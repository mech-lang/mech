use mech_syntax::document::ast::ModuleImportSyntax;
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2e_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpectedSyntax, FixApplicability, ParseConfig, RecoveryAction, Revision,
    RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
    lower_legacy_module_import,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(927), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2e_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2E direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn diagnostic_codes(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
) -> Vec<String> {
    parsed
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str().to_owned())
        .collect()
}

fn assert_committed(rule: RuleId, input: &str, expected_codes: &[&str]) {
    let parsed = parse(input, rule);
    assert!(parsed.matched, "{rule:?} on {input:?}");
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{input:?}");
    assert_eq!(
        diagnostic_codes(&parsed),
        expected_codes
            .iter()
            .map(|code| (*code).to_owned())
            .collect::<Vec<_>>(),
        "{input:?}",
    );
}

#[test]
fn intrinsic_segments_commit_to_a_local_missing_name() {
    for (rule, input) in [
        (rules::MODULE_IMPORT_INTRINSIC_SEGMENT, "_"),
        (rules::MODULE_IMPORT_PATH, "math/_"),
        (rules::MODULE_IMPORT, "+> math/_"),
    ] {
        let parsed = parse(input, rule);
        assert!(parsed.matched, "{input:?}");
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{input:?}");
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(
            diagnostic.code.as_str(),
            "syntax/missing-module-import-intrinsic-name",
            "{input:?}",
        );
        assert_eq!(
            diagnostic.expected,
            vec![ExpectedSyntax::Production(
                "module-import-name-segment".into()
            )],
        );
        assert!(diagnostic.fixes.is_empty());
        assert!(find_node(&parsed.syntax(), SyntaxKind::Missing).is_some());
    }
}

#[test]
fn context_alias_and_completed_alias_operator_have_local_recovery() {
    assert_committed(
        rules::MODULE_IMPORT,
        "+> @",
        &["syntax/missing-module-import-context-alias"],
    );

    assert_committed(
        rules::MODULE_IMPORT,
        "+> alias :=",
        &["syntax/missing-module-import-alias-target"],
    );
    assert_committed(
        rules::MODULE_IMPORT,
        "+> alias := math",
        &["syntax/missing-module-import-aliased-item-separator"],
    );
    assert_committed(
        rules::MODULE_IMPORT,
        "+> alias := math/",
        &["syntax/missing-module-import-aliased-item"],
    );
    assert_committed(
        rules::MODULE_IMPORT,
        "+> @ctx :=",
        &["syntax/missing-module-import-alias-target"],
    );

    let plain = parse("+> alias", rules::MODULE_IMPORT);
    assert!(plain.is_strictly_clean());
    assert_eq!(plain.outcome, CanonicalRuleOutcome::Matched);
    assert!(find_node(&plain.syntax(), SyntaxKind::ModuleOnlyImport).is_some());
}

#[test]
fn context_alias_prefixes_commit_without_inventing_a_pre_operator_diagnostic() {
    let bare_context = parse("+> @ctx", rules::MODULE_IMPORT);
    assert!(bare_context.matched);
    assert_eq!(bare_context.outcome, CanonicalRuleOutcome::Committed);
    assert!(bare_context.diagnostics.is_empty());

    let slash_context = parse("+> @ctx/path := math/sin", rules::MODULE_IMPORT);
    assert!(slash_context.matched);
    assert_eq!(slash_context.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(slash_context.consumed.end, TextSize(7));
    let node = find_node(&slash_context.syntax(), SyntaxKind::ModuleImport).unwrap();
    let syntax = ModuleImportSyntax::cast(node).unwrap();
    assert!(
        lower_legacy_module_import(&syntax).is_err(),
        "a slash-continuing context alias must not lower as a valid module import"
    );
}

#[test]
fn module_suffix_and_group_commit_points_remain_local() {
    assert_committed(
        rules::MODULE_IMPORT,
        "+> math/",
        &["syntax/missing-module-import-suffix"],
    );
    assert_committed(
        rules::MODULE_IMPORT,
        "+> math/{",
        &[
            "syntax/missing-module-import-group-item",
            "syntax/unclosed-module-import-group",
        ],
    );
    assert_committed(
        rules::MODULE_IMPORT,
        "+> math/{}",
        &["syntax/missing-module-import-group-item"],
    );
    assert_committed(
        rules::MODULE_IMPORT,
        "+> math/{sin,",
        &[
            "syntax/missing-module-import-group-item",
            "syntax/unclosed-module-import-group",
        ],
    );
}

#[test]
fn missing_group_closer_inserts_a_zero_width_brace_with_a_fix() {
    let parsed = parse("+> math/{sin", rules::MODULE_IMPORT);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    let diagnostic = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "syntax/unclosed-module-import-group")
        .unwrap();
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Token(SyntaxKind::RightBrace)]
    );
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: ExpectedSyntax::Token(SyntaxKind::RightBrace),
            at: TextSize(12),
        })
    );
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(
        diagnostic.fixes[0].applicability,
        FixApplicability::MachineApplicable
    );

    let missing = find_node(&parsed.syntax(), SyntaxKind::Missing).unwrap();
    let braces = missing
        .tokens()
        .into_iter()
        .filter(|token| token.kind() == SyntaxKind::RightBrace)
        .collect::<Vec<_>>();
    assert_eq!(braces.len(), 1);
    assert_eq!(braces[0].range(), TextRange::empty(TextSize(12)));
    assert!(braces[0].flags().contains(TokenFlags::MISSING));
}

#[test]
fn shared_sigil_alone_never_commits_recovery() {
    for input in ["", "+", "+>", "+> ", "+>\nmath", "+> /", "+⟩ math"] {
        let parsed = parse(input, rules::MODULE_IMPORT);
        assert!(!parsed.matched, "{input:?}");
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{input:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{input:?}"
        );
        assert!(parsed.diagnostics.is_empty(), "{input:?}");
    }
}
