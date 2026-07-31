use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2f_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(934), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2f_rule_for_test(source(text), rule, ParseConfig::default()).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn assert_clean_prefix(rule: RuleId, input: &str, expected: &str, kind: SyntaxKind) {
    let parsed = parse(input, rule);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{input:?}");
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.source.text(parsed.consumed).unwrap(), expected);
    assert!(find_node(&parsed.syntax(), kind).is_some());
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
}

fn assert_no_match(rule: RuleId, input: &str) {
    let parsed = parse(input, rule);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{input:?}");
    assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn export_declaration_retains_its_distinct_whitespace_contract() {
    for input in ["<+ value", "<+\tvalue", "<+\nvalue"] {
        assert_clean_prefix(
            rules::EXPORT_DECLARATION,
            input,
            input,
            SyntaxKind::ExportDeclaration,
        );
    }
    for input in ["<+value", "<+\u{00a0}value", "<+\u{2009}value"] {
        assert_no_match(rules::EXPORT_DECLARATION, input);
    }
}

#[test]
fn context_base_forms_preserve_their_legacy_boundaries() {
    for input in ["fs://workspace", "1.0://a_b/path", "-://x", ".://x"] {
        assert_clean_prefix(
            rules::CONTEXT_BASE_RESOURCE_URI,
            input,
            input,
            SyntaxKind::ContextBaseResourceUri,
        );
    }
    for input in ["fs://", "://x"] {
        assert_no_match(rules::CONTEXT_BASE_RESOURCE_URI, input);
    }
    for input in ["@main", "@main/sub", "@💡"] {
        let parsed = parse(input, rules::CONTEXT_BASE_CONTEXT);
        assert!(parsed.is_strictly_clean(), "{input:?}");
    }
}

#[test]
fn capability_paths_validate_complete_wildcard_placement() {
    for input in ["users", "users/read", "users/*", "///*", "_/*"] {
        assert_clean_prefix(
            rules::CONTEXT_CAPABILITY_PATH,
            input,
            input,
            SyntaxKind::ContextCapabilityPath,
        );
    }
    for input in ["*", "/*", "foo*", "foo/*/bar", "foo/**"] {
        assert_no_match(rules::CONTEXT_CAPABILITY_PATH, input);
    }
    for (input, expected) in [("**", "*"), ("*/foo", "*")] {
        assert_clean_prefix(
            rules::CONTEXT_CAPABILITY_SCOPE,
            input,
            expected,
            SyntaxKind::ContextCapabilityScope,
        );
    }
}

#[test]
fn capability_groups_are_all_or_nothing_suffixes() {
    for (input, expected) in [
        (
            "@users := @main{:read(*), :write(users/*)}",
            "@users := @main{:read(*), :write(users/*)}",
        ),
        ("@users := @main{:read(*),}", "@users := @main{:read(*),}"),
        ("@users := @main{}", "@users := @main"),
        ("@users := @main {:read(*)}", "@users := @main"),
        ("@users := @main{:read(foo*)}", "@users := @main"),
        ("@users := @main{:read(*)", "@users := @main"),
    ] {
        assert_clean_prefix(
            rules::CONTEXT_DECLARATION,
            input,
            expected,
            SyntaxKind::ContextDeclaration,
        );
    }
}

#[test]
fn all_declaration_rules_have_a_direct_contract() {
    let cases = [
        (
            rules::EXPORT_DECLARATION,
            "<+ value",
            SyntaxKind::ExportDeclaration,
        ),
        (
            rules::CONTEXT_DECLARATION,
            "@ui := fs://workspace",
            SyntaxKind::ContextDeclaration,
        ),
        (
            rules::CONTEXT_BASE_CONTEXT,
            "@main",
            SyntaxKind::ContextBaseContext,
        ),
        (
            rules::CONTEXT_BASE_RESOURCE_URI,
            "fs://workspace",
            SyntaxKind::ContextBaseResourceUri,
        ),
        (
            rules::CONTEXT_CAPABILITY_DECLARATION,
            ":read(*)",
            SyntaxKind::ContextCapabilityDeclaration,
        ),
        (
            rules::CONTEXT_CAPABILITY_PATH,
            "users",
            SyntaxKind::ContextCapabilityPath,
        ),
        (
            rules::CONTEXT_CAPABILITY_SCOPE,
            "*",
            SyntaxKind::ContextCapabilityScope,
        ),
    ];
    assert_eq!(cases.len(), 7);
    for (rule, input, kind) in cases {
        assert_clean_prefix(rule, input, input, kind);
    }
    let token = parse("*", rules::CONTEXT_CAPABILITY_PATH_TOKEN);
    assert!(token.is_strictly_clean());
}
