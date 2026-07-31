use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2e_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(926), Revision(0), text).unwrap()
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

fn assert_match(rule: RuleId, input: &str, expected_kind: Option<SyntaxKind>) {
    let parsed = parse(input, rule);
    assert_eq!(parsed.rule, rule, "{input:?}");
    assert!(parsed.matched, "{rule:?} did not accept {input:?}");
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(
        parsed.consumed,
        TextRange::new(TextSize::ZERO, parsed.source.byte_len()),
        "{rule:?} did not consume {input:?}",
    );
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        input,
        "{rule:?} did not preserve {input:?}",
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    if let Some(kind) = expected_kind {
        assert!(
            find_node(&parsed.syntax(), kind).is_some(),
            "{rule:?} did not emit {kind:?} for {input:?}",
        );
    }
}

fn assert_no_match(rule: RuleId, input: &str) {
    let parsed = parse(input, rule);
    assert!(!parsed.matched, "{rule:?} unexpectedly accepted {input:?}");
    assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
    assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
}

#[test]
fn every_phase_2e_rule_accepts_its_direct_contract() {
    let cases = [
        (
            rules::MODULE_IMPORT_NAME_SEGMENT,
            "math",
            Some(SyntaxKind::ModuleImportNameSegment),
        ),
        (
            rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
            "_math",
            Some(SyntaxKind::ModuleImportIntrinsicSegment),
        ),
        (
            rules::MODULE_IMPORT_PATH_SEGMENT,
            "_math",
            Some(SyntaxKind::ModuleImportPathSegment),
        ),
        (
            rules::MODULE_IMPORT_PATH,
            "math/trig/_sin",
            Some(SyntaxKind::ModuleImportPath),
        ),
        (
            rules::MODULE_IMPORT_ALIAS_SEGMENT,
            "alias",
            Some(SyntaxKind::ModuleImportAliasSegment),
        ),
        (
            rules::MODULE_IMPORT_ALIAS_PATH,
            "alias/path",
            Some(SyntaxKind::ModuleImportAliasPath),
        ),
        (
            rules::MODULE_IMPORT_VALUE_ALIAS,
            "alias/path",
            Some(SyntaxKind::ModuleImportValueAlias),
        ),
        (
            rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
            "ctx-2",
            Some(SyntaxKind::ContextImportAliasSegment),
        ),
        (
            rules::MODULE_IMPORT_CONTEXT_ALIAS,
            "@ctx",
            Some(SyntaxKind::ModuleImportContextAlias),
        ),
        (
            rules::MODULE_IMPORT_ALIAS,
            "@ctx",
            Some(SyntaxKind::ModuleImportAlias),
        ),
        (rules::MODULE_ROOT, "math", Some(SyntaxKind::ModuleRoot)),
        (rules::IMPORT_ALIAS_OPERATOR, " := ", None),
        (rules::IMPORT_GROUP_SEPARATOR, ",", None),
        (
            rules::IMPORT_GROUP_ITEM,
            "trig/sin",
            Some(SyntaxKind::ImportGroupItem),
        ),
        (
            rules::IMPORT_GROUP_ITEMS,
            "sin,cos",
            Some(SyntaxKind::ImportGroupItems),
        ),
        (
            rules::ALIASED_ITEM_IMPORT,
            "alias := math/sin",
            Some(SyntaxKind::AliasedItemImport),
        ),
        (
            rules::MODULE_SUFFIX_IMPORT,
            "math/*",
            Some(SyntaxKind::ModuleSuffixImport),
        ),
        (
            rules::MODULE_ONLY_IMPORT,
            "math",
            Some(SyntaxKind::ModuleOnlyImport),
        ),
        (
            rules::MODULE_IMPORT,
            "+> math",
            Some(SyntaxKind::ModuleImport),
        ),
    ];
    assert_eq!(cases.len(), 19);
    for (rule, input, kind) in cases {
        assert_match(rule, input, kind);
    }
}

#[test]
fn complete_module_import_forms_are_lossless_and_closed() {
    for input in [
        "+>math",
        "+> math",
        "+> math/sin",
        "+> math/trig/sin",
        "+> math/*",
        "+> math/{sin,cos}",
        "+> math/{sin cos}",
        "+> math/{sin\ncos}",
        "+> math/{trig/sin,stats/mean}",
        "+> math/_intrinsic",
        "+> math/path/_intrinsic",
        "+> alias := math/sin",
        "+> alias/path := math/sin",
        "+> @ctx := math/sin",
        "+> 💡",
        "+> 💡/run",
        "+> math/emoji-name",
    ] {
        assert_match(rules::MODULE_IMPORT, input, Some(SyntaxKind::ModuleImport));
    }
}

#[test]
fn shared_import_sigil_has_only_the_module_import_prefix_behavior_here() {
    for input in [
        "+>math",
        "+> math",
        "+>\tmath",
        "+>\u{00a0}math",
        "+>\u{2009}math",
    ] {
        assert_match(rules::MODULE_IMPORT, input, Some(SyntaxKind::ModuleImport));
    }

    for input in ["+⟩ math", "+>\nmath"] {
        assert_no_match(rules::MODULE_IMPORT, input);
    }

    let source_import_prefix = parse("+> foo.mec", rules::MODULE_IMPORT);
    assert!(source_import_prefix.is_strictly_clean());
    assert_eq!(source_import_prefix.consumed.end, TextSize(6));
    assert_eq!(
        source_import_prefix
            .source
            .text(source_import_prefix.consumed)
            .unwrap(),
        "+> foo"
    );
    assert!(find_node(&source_import_prefix.syntax(), SyntaxKind::ModuleOnlyImport).is_some());
}

#[test]
fn direct_repetition_rewinds_incomplete_pairs_without_local_recovery() {
    let path = parse("math/", rules::MODULE_IMPORT_PATH);
    assert!(path.is_strictly_clean());
    assert_eq!(path.consumed.end, TextSize(4));
    assert_eq!(path.source.text(path.consumed).unwrap(), "math");

    let group = parse("sin,", rules::IMPORT_GROUP_ITEMS);
    assert_eq!(group.outcome, CanonicalRuleOutcome::Matched);
    assert!(group.is_strictly_clean());
    assert_eq!(group.consumed.end, TextSize(3));
    assert_eq!(group.source.text(group.consumed).unwrap(), "sin");
    assert!(group.diagnostics.is_empty());
}
