use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, NodeFlags, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c5), Revision(0), text).unwrap()
}

fn contains_kind(node: &SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected || node.children().any(|child| contains_kind(&child, expected))
}

fn committed(rule: RuleId, text: &str, parent: SyntaxKind) {
    let parsed =
        parse_canonical_phase_2i_rule_for_test(source(text), rule, ParseConfig::default()).unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
    assert!(!parsed.diagnostics.is_empty(), "{text:?}");
    assert!(contains_kind(&parsed.syntax(), parent), "{text:?}");
    assert!(parsed.root.flags.intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING
    ));
}

#[test]
fn nesting_limit_finishes_balanced_without_stack_overflow() {
    let limits = ParseLimits {
        max_nesting: 8,
        ..ParseLimits::default()
    };
    let text = format!("{}1{}", "(".repeat(64), ")".repeat(64));
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(!parsed.diagnostics.is_empty());
    assert!(parsed.root.flags.intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING
    ));
}

#[test]
fn fuel_is_a_hard_limit_and_resource_completion_is_balanced() {
    let limits = ParseLimits {
        fuel: 64,
        ..ParseLimits::default()
    };
    let text = core::iter::repeat_n("1", 4_096)
        .collect::<Vec<_>>()
        .join(" + ");
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    assert!(parsed.stats.parser_steps <= limits.fuel);
    assert_eq!(parsed.stats.parser_steps, limits.fuel);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(!parsed.diagnostics.is_empty());
}

#[test]
fn event_budget_is_a_hard_limit_and_resource_completion_is_balanced() {
    let limits = ParseLimits {
        max_events: 32,
        ..ParseLimits::default()
    };
    let text = format!("[{}]", "1 ".repeat(4_096));
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(!parsed.diagnostics.is_empty());
}

#[test]
fn later_committed_children_are_retained_by_every_recursive_repetition() {
    for (rule, text, parent) in [
        (rules::MAP, "{1: 2, 3: \"unterminated", SyntaxKind::Map),
        (
            rules::RECORD,
            "{a: 1, b: \"unterminated",
            SyntaxKind::Record,
        ),
        (rules::SET, "{1, \"unterminated", SyntaxKind::Set),
        (rules::TUPLE, "(1, \"unterminated", SyntaxKind::Tuple),
        (
            rules::ARGUMENT_LIST,
            "(1, \"unterminated",
            SyntaxKind::ArgumentList,
        ),
        (
            rules::INLINE_TABLE_ROW,
            "1 \"unterminated",
            SyntaxKind::InlineTableRow,
        ),
        (
            rules::INLINE_TABLE,
            "|a<u8>|1|2 \"unterminated",
            SyntaxKind::InlineTable,
        ),
        (rules::MATRIX, "[1 \"unterminated", SyntaxKind::Matrix),
        (rules::MATRIX, "[\n1\n\"unterminated", SyntaxKind::Matrix),
    ] {
        committed(rule, text, parent);
    }
}

#[test]
fn low_fuel_in_later_recursive_children_retains_each_parent() {
    let tail = core::iter::repeat_n("1", 512)
        .collect::<Vec<_>>()
        .join(" + ");
    let cases = [
        (rules::MAP, format!("{{1: 2, 3: {tail}}}"), SyntaxKind::Map),
        (
            rules::RECORD,
            format!("{{a: 1, b: {tail}}}"),
            SyntaxKind::Record,
        ),
        (
            rules::INLINE_TABLE_ROW,
            format!("1 {tail}|"),
            SyntaxKind::InlineTableRow,
        ),
        (
            rules::MATRIX,
            format!("[\n1\n{tail}\n]"),
            SyntaxKind::Matrix,
        ),
    ];
    for (rule, text, parent) in cases {
        let limits = ParseLimits {
            fuel: 256,
            ..ParseLimits::default()
        };
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source(&text), rule, ParseConfig { limits })
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{rule:?}");
        assert_eq!(parsed.stats.parser_steps, limits.fuel, "{rule:?}");
        assert!(!parsed.diagnostics.is_empty(), "{rule:?}");
        assert!(contains_kind(&parsed.syntax(), parent), "{rule:?}");
    }
}
