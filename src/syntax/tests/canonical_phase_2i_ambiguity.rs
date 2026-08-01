use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2c_rule_for_test,
    parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, compact_debug_tree,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c1), Revision(0), text).unwrap()
}

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let parsed = parse_raw(rule, text);
    assert!(parsed.is_strictly_clean(), "{rule:?} on {text:?}");
    assert_eq!(parsed.consumed.end, TextSize(text.len() as u32));
    parsed
}

fn parse_raw(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2i_rule_for_test(source(text), rule, ParseConfig::default()).unwrap()
}

fn assert_outer_chain(rule: RuleId, text: &str, expected: &[SyntaxKind]) {
    let parsed = parse(rule, text);
    let mut node = parsed.syntax();
    for expected_kind in expected {
        let children = node.children().collect::<Vec<_>>();
        assert_eq!(
            children.len(),
            1,
            "unexpected outer hierarchy for {text:?}:\n{}",
            compact_debug_tree(&parsed.syntax())
        );
        assert_eq!(
            children[0].kind(),
            *expected_kind,
            "wrong selection for {text:?}:\n{}",
            compact_debug_tree(&parsed.syntax())
        );
        node = children[0].clone();
    }
}

fn contains_kind(node: &SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected || node.children().any(|child| contains_kind(&child, expected))
}

#[test]
fn shared_prefixes_select_the_exact_outer_structure() {
    let cases: &[(RuleId, &str, &[SyntaxKind])] = &[
        (
            rules::FACTOR,
            "(1)",
            &[SyntaxKind::Factor, SyntaxKind::ParentheticalExpression],
        ),
        (
            rules::FACTOR,
            "(1, 2)",
            &[SyntaxKind::Factor, SyntaxKind::Structure, SyntaxKind::Tuple],
        ),
        (
            rules::FACTOR,
            "()",
            &[SyntaxKind::Factor, SyntaxKind::Structure, SyntaxKind::Tuple],
        ),
        (
            rules::FACTOR,
            "[1 2]",
            &[
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::Matrix,
            ],
        ),
        (
            rules::FACTOR,
            "[x | x <- xs]",
            &[SyntaxKind::Factor, SyntaxKind::MatrixComprehension],
        ),
        (
            rules::FACTOR,
            "{}",
            &[
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::EmptySet,
            ],
        ),
        (
            rules::FACTOR,
            "{:}",
            &[
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::EmptyMap,
            ],
        ),
        (
            rules::FACTOR,
            "{1: 2}",
            &[SyntaxKind::Factor, SyntaxKind::Structure, SyntaxKind::Map],
        ),
        (
            rules::FACTOR,
            "{a: 2}",
            &[
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::Record,
            ],
        ),
        (
            rules::FACTOR,
            "{a + b: 2}",
            &[SyntaxKind::Factor, SyntaxKind::Structure, SyntaxKind::Map],
        ),
        (
            rules::FACTOR,
            "{a, b}",
            &[SyntaxKind::Factor, SyntaxKind::Structure, SyntaxKind::Set],
        ),
        (
            rules::FACTOR,
            ":some",
            &[
                SyntaxKind::Factor,
                SyntaxKind::Literal,
                SyntaxKind::AtomLiteral,
            ],
        ),
        (
            rules::FACTOR,
            ":some(value)",
            &[
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::TupleStruct,
            ],
        ),
        (
            rules::FACTOR,
            "foo",
            &[SyntaxKind::Factor, SyntaxKind::Variable],
        ),
        (
            rules::FACTOR,
            "foo(value)",
            &[SyntaxKind::Factor, SyntaxKind::FunctionCall],
        ),
        (
            rules::FACTOR,
            "foo[1]",
            &[SyntaxKind::Factor, SyntaxKind::Slice],
        ),
        (
            rules::FACTOR,
            "foo.field",
            &[SyntaxKind::Factor, SyntaxKind::Slice],
        ),
        (
            rules::FACTOR,
            "@ctx/path",
            &[SyntaxKind::Factor, SyntaxKind::Variable],
        ),
        (
            rules::FACTOR,
            "@ctx/path[1]",
            &[SyntaxKind::Factor, SyntaxKind::Slice],
        ),
        (
            rules::EXPRESSION,
            "#machine",
            &[SyntaxKind::Expression, SyntaxKind::FsmPipe],
        ),
        (
            rules::EXPRESSION,
            "#machine() -> :next",
            &[SyntaxKind::Expression, SyntaxKind::FsmPipe],
        ),
        (
            rules::EXPRESSION,
            "1..10",
            &[SyntaxKind::Expression, SyntaxKind::RangeExpression],
        ),
        (
            rules::EXPRESSION,
            "1..10..2",
            &[SyntaxKind::Expression, SyntaxKind::RangeExpression],
        ),
        (
            rules::EXPRESSION,
            "{x | x <- xs}",
            &[SyntaxKind::Expression, SyntaxKind::SetComprehension],
        ),
        (
            rules::EXPRESSION,
            "[x | x <- xs]",
            &[SyntaxKind::Expression, SyntaxKind::MatrixComprehension],
        ),
        (
            rules::EXPRESSION,
            "╭1╯",
            &[
                SyntaxKind::Expression,
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::Matrix,
            ],
        ),
        (
            rules::EXPRESSION,
            "|a: 1|",
            &[
                SyntaxKind::Expression,
                SyntaxKind::Factor,
                SyntaxKind::Structure,
                SyntaxKind::Record,
            ],
        ),
        (
            rules::EXPRESSION,
            "1 + 2 * 3",
            &[SyntaxKind::Expression, SyntaxKind::AdditiveExpression],
        ),
    ];
    for (rule, text, hierarchy) in cases {
        assert_outer_chain(*rule, text, hierarchy);
    }

    let precedence = parse(rules::EXPRESSION, "1 + 2 * 3");
    assert!(contains_kind(
        &precedence.syntax(),
        SyntaxKind::MultiplicativeExpression
    ));
    let matched = parse(rules::EXPRESSION, "x ? | * => 1");
    let expression = matched.syntax().children().next().unwrap();
    assert_eq!(expression.kind(), SyntaxKind::Expression);
    assert!(contains_kind(&expression, SyntaxKind::MatchArm));

    for text in ["x? | * => 1", "x ? | * => 1", "x\t?\n| * => 1"] {
        let matched = parse(rules::EXPRESSION, text);
        assert!(contains_kind(&matched.syntax(), SyntaxKind::MatchArm));
    }

    let set_factor = parse_raw(rules::FACTOR, "{x | x <- xs}");
    assert_eq!(set_factor.outcome, CanonicalRuleOutcome::NoMatch);
    assert_eq!(set_factor.consumed, TextRange::empty(TextSize::ZERO));

    for (text, prefix, kind) in [
        (
            "{x | x <- xs} + 1",
            "{x | x <- xs}",
            SyntaxKind::SetComprehension,
        ),
        (
            "[x | x <- xs] + 1",
            "[x | x <- xs]",
            SyntaxKind::MatrixComprehension,
        ),
    ] {
        let parsed = parse_raw(rules::EXPRESSION, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
        assert_eq!(parsed.consumed.end, TextSize(prefix.len() as u32));
        assert!(contains_kind(&parsed.syntax(), kind));
        assert!(!contains_kind(
            &parsed.syntax(),
            SyntaxKind::AdditiveExpression
        ));
    }

    for (text, kind) in [
        ("{1} ∪ {2}", SyntaxKind::SetExpression),
        ("[1] + [2]", SyntaxKind::AdditiveExpression),
    ] {
        let parsed = parse(rules::EXPRESSION, text);
        assert!(contains_kind(&parsed.syntax(), kind));
    }
}

#[test]
fn lower_level_rules_retain_their_own_prefix_contracts() {
    assert_outer_chain(rules::TUPLE, "(1)", &[SyntaxKind::Tuple]);
    assert_outer_chain(
        rules::PARENTHETICAL_TERM,
        "(1)",
        &[SyntaxKind::ParentheticalExpression],
    );

    let atom = parse_canonical_phase_2c_rule_for_test(
        source(":some(value)"),
        rules::ATOM,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(atom.outcome, CanonicalRuleOutcome::Matched);
    assert_eq!(atom.consumed, TextRange::new(TextSize::ZERO, TextSize(5)));

    for (rule, text, end) in [
        (rules::VAR, "foo(value)", 3),
        (rules::KIND_SCALAR, "u8:", 2),
    ] {
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source(text), rule, ParseConfig::default())
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
        assert_eq!(
            parsed.consumed,
            TextRange::new(TextSize::ZERO, TextSize(end))
        );
    }
}
