use std::fs;
use std::path::{Path, PathBuf};

use mech_core::{Grammar, GrammarExpression, Token};
use mech_syntax::document::parser::canonical_rule_name;
use mech_syntax::document::{
    DocumentId, FixApplicability, NodeFlags, ParseConfig, RecoveryAction, Revision, SyntaxKind,
    SyntaxNode, SyntaxSnapshot, TextSnapshot, compact_debug_tree, lower_legacy_grammar,
    parse_canonical_grammar, reconstruct_source, validate_lossless,
};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/canonical/grammar")
}

fn fixture(directory: &str, name: &str) -> String {
    fs::read_to_string(fixture_root().join(directory).join(name))
        .unwrap_or_else(|error| panic!("read {directory}/{name}: {error}"))
}

fn parse(text: &str) -> SyntaxSnapshot {
    parse_canonical_grammar(
        TextSnapshot::new(DocumentId(43), Revision(7), text).unwrap(),
        ParseConfig::default(),
    )
}

fn nodes_of_kind(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut nodes = Vec::new();
    if root.kind() == kind {
        nodes.push(root.clone());
    }
    for child in root.children() {
        nodes.extend(nodes_of_kind(&child, kind));
    }
    nodes
}

fn assert_lossless(text: &str, snapshot: &SyntaxSnapshot) {
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
}

fn assert_tree_fixture(name: &str, snapshot: &SyntaxSnapshot) {
    let actual = compact_debug_tree(&snapshot.syntax());
    let expected = fixture("trees", name);
    assert_eq!(actual, expected);
}

fn normalize_token_source(token: &mut Token) {
    token.src_range = Default::default();
}

fn normalize_expression_source(expression: &mut GrammarExpression) {
    match expression {
        GrammarExpression::Choice(items) | GrammarExpression::Sequence(items) => {
            for item in items {
                normalize_expression_source(item);
            }
        }
        GrammarExpression::Definition(identifier) => normalize_token_source(&mut identifier.name),
        GrammarExpression::Group(item)
        | GrammarExpression::Not(item)
        | GrammarExpression::Optional(item)
        | GrammarExpression::Peek(item)
        | GrammarExpression::Repeat0(item)
        | GrammarExpression::Repeat1(item) => normalize_expression_source(item),
        GrammarExpression::List(first, second) => {
            normalize_expression_source(first);
            normalize_expression_source(second);
        }
        GrammarExpression::Range(start, end) => {
            normalize_token_source(start);
            normalize_token_source(end);
        }
        GrammarExpression::Terminal(token) => normalize_token_source(token),
    }
}

fn normalize_grammar_source(mut grammar: Grammar) -> Grammar {
    for rule in &mut grammar.rules {
        normalize_token_source(&mut rule.name.name);
        normalize_expression_source(&mut rule.expr);
    }
    grammar
}

fn assert_legacy_parity(text: &str, snapshot: &SyntaxSnapshot) {
    let canonical = lower_legacy_grammar(snapshot).unwrap();
    let legacy = mech_syntax::parse_grammar(text).unwrap();
    assert_eq!(
        normalize_grammar_source(canonical),
        normalize_grammar_source(legacy)
    );
}

fn assert_canonical_diagnostics(snapshot: &SyntaxSnapshot) {
    assert!(!snapshot.diagnostics.is_empty());
    for diagnostic in snapshot.diagnostics.iter() {
        let rule = diagnostic
            .rule
            .unwrap_or_else(|| panic!("{} has no canonical rule", diagnostic.code.as_str()));
        assert!(
            canonical_rule_name(rule).is_some(),
            "{} uses an unregistered canonical rule",
            diagnostic.code.as_str()
        );
        assert_eq!(
            diagnostic.context,
            None,
            "{} leaked a prototype context",
            diagnostic.code.as_str()
        );
        assert!(
            diagnostic.recovery.is_some(),
            "{} has no recovery action",
            diagnostic.code.as_str()
        );
        for fix in &diagnostic.fixes {
            assert!(
                fix.edits
                    .iter()
                    .all(|edit| snapshot.source.full_range().contains_range(edit.delete)),
                "{} has an out-of-bounds fix",
                diagnostic.code.as_str()
            );
            snapshot
                .source
                .apply_edits(&fix.edits)
                .unwrap_or_else(|error| {
                    panic!("{} has an invalid fix: {error}", diagnostic.code.as_str())
                });
        }
    }
}

#[test]
fn accepted_expression_fixture_is_lossless_and_has_legacy_parity() {
    let text = fixture("accepted", "core-expressions.mec");
    let snapshot = parse(&text);
    assert!(
        snapshot.diagnostics.is_empty(),
        "{:#?}",
        snapshot.diagnostics.as_slice()
    );
    assert_lossless(&text, &snapshot);
    assert_legacy_parity(&text, &snapshot);
}

#[test]
fn grammar_filtering_fixture_is_lossless_and_has_legacy_parity() {
    let text = fixture("accepted", "filtered-trivia.mec");
    let snapshot = parse(&text);
    assert!(
        snapshot.diagnostics.is_empty(),
        "{:#?}",
        snapshot.diagnostics.as_slice()
    );
    assert_lossless(&text, &snapshot);
    assert_legacy_parity(&text, &snapshot);
}

#[test]
fn terminal_with_filtered_space_has_an_exact_tree_and_legacy_parity() {
    let text = fixture("accepted", "terminal-with-space.mec");
    let snapshot = parse(&text);
    assert!(snapshot.diagnostics.is_empty());
    assert_lossless(&text, &snapshot);
    assert_legacy_parity(&text, &snapshot);
    assert_tree_fixture("terminal-with-space.tree", &snapshot);
}

#[test]
fn malformed_fixtures_have_bounded_structural_recovery() {
    let manifest = fixture("diagnostics", "recovery-cases.tsv");
    for (index, line) in manifest.lines().enumerate().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            4,
            "invalid recovery manifest row {}",
            index + 1
        );
        let name = fields[0];
        let expected_code = fields[1];
        let expected_rule = fields[2];
        let machine_fix = fields[3] == "machine-applicable";
        let text = fixture("malformed", name);
        let snapshot = parse(&text);

        assert_lossless(&text, &snapshot);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == expected_code),
            "{name} did not emit {expected_code}: {:#?}",
            snapshot.diagnostics.as_slice()
        );
        let primary = snapshot
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code.as_str() == expected_code)
            .unwrap();
        assert_eq!(
            primary.rule.and_then(canonical_rule_name),
            Some(expected_rule),
            "{name} attributed {expected_code} to the wrong canonical rule"
        );
        assert!(
            snapshot.diagnostics.len() <= 3,
            "{name} produced an unbounded cascade: {:#?}",
            snapshot.diagnostics.as_slice()
        );
        assert_canonical_diagnostics(&snapshot);
        assert!(
            snapshot
                .root
                .flags
                .intersects(NodeFlags::CONTAINS_ERROR | NodeFlags::CONTAINS_MISSING),
            "{name} did not retain structural recovery"
        );
        assert!(
            lower_legacy_grammar(&snapshot).is_err(),
            "{name} unexpectedly lowered despite syntax diagnostics"
        );

        if machine_fix {
            let diagnostic = snapshot
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code.as_str() == expected_code)
                .unwrap();
            assert!(
                diagnostic.fixes.iter().any(|fix| {
                    fix.applicability == FixApplicability::MachineApplicable
                        && !fix.edits.is_empty()
                }),
                "{name} did not provide a machine-applicable fix for {expected_code}"
            );
            assert!(matches!(
                diagnostic.recovery,
                Some(RecoveryAction::Insert { .. })
            ));
        }
    }
}

#[test]
fn unexpected_source_is_skipped_and_retained() {
    let text = fixture("malformed", "unexpected-token.mec");
    let snapshot = parse(&text);
    assert_lossless(&text, &snapshot);
    let diagnostic = snapshot
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "syntax/unexpected-grammar-token")
        .expect("unexpected grammar token diagnostic");
    assert!(matches!(
        diagnostic.recovery,
        Some(RecoveryAction::Skip { .. })
    ));
    assert!(
        nodes_of_kind(&snapshot.syntax(), SyntaxKind::Error)
            .iter()
            .any(|node| node.text().unwrap().contains('@'))
    );
}

#[test]
fn a_malformed_rule_does_not_hide_later_independent_rules() {
    for name in ["later-rule-survives.mec", "missing-semicolon.mec"] {
        let text = fixture("malformed", name);
        let snapshot = parse(&text);
        assert_lossless(&text, &snapshot);
        assert_canonical_diagnostics(&snapshot);

        let rule_text = nodes_of_kind(&snapshot.syntax(), SyntaxKind::GrammarRule)
            .into_iter()
            .map(|node| node.text().unwrap())
            .collect::<Vec<_>>();
        assert!(
            rule_text
                .iter()
                .any(|rule| rule.contains("second") || rule.contains("next")),
            "{name} lost its later rule: {rule_text:?}"
        );
    }
}

#[test]
fn unclosed_terminal_synchronizes_before_a_later_rule() {
    let text = "first := \"abc ;\nsecond := \"b\";";
    let snapshot = parse(text);
    assert_lossless(text, &snapshot);
    assert_canonical_diagnostics(&snapshot);
    assert!(
        snapshot.diagnostics.len() <= 2,
        "unclosed terminal produced an unbounded cascade: {:#?}",
        snapshot.diagnostics.as_slice()
    );
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "syntax/unclosed-grammar-terminal")
    );
    assert!(lower_legacy_grammar(&snapshot).is_err());

    let rule_text = nodes_of_kind(&snapshot.syntax(), SyntaxKind::GrammarRule)
        .into_iter()
        .map(|node| node.text().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rule_text.len(), 2, "later rule was not recovered: {rule_text:?}");
    assert!(
        rule_text[1].contains("second"),
        "later rule was not recovered: {rule_text:?}"
    );
}

#[test]
fn grammar_restart_metadata_covers_the_root_grammar_and_semicolon_rules() {
    let text = "first:=\"a\";second:=\"b\";";
    let snapshot = parse(text);
    assert!(snapshot.diagnostics.is_empty());
    let root = snapshot.syntax();

    for kind in [SyntaxKind::GrammarDocument, SyntaxKind::Grammar] {
        let node = nodes_of_kind(&root, kind)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("missing {kind:?}"));
        let restart = snapshot
            .restarts
            .get(node.id())
            .unwrap_or_else(|| panic!("missing restart for {kind:?}"));
        assert_eq!(restart.mode, mech_syntax::document::RestartMode::Grammar);
        assert_eq!(restart.range, node.range());
    }

    let rules = nodes_of_kind(&root, SyntaxKind::GrammarRule);
    assert_eq!(rules.len(), 2);
    for rule in rules {
        let restart = snapshot
            .restarts
            .get(rule.id())
            .expect("missing grammar-rule restart");
        assert_eq!(restart.mode, mech_syntax::document::RestartMode::Grammar);
        assert_eq!(restart.range, rule.range());
        assert_eq!(
            snapshot
                .source
                .byte_at(restart.range.end - mech_syntax::document::TextSize(1)),
            Some(b';'),
            "grammar-rule restart did not end at its semicolon"
        );
    }
}
