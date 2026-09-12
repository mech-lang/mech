use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, NodeFlags, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange,
    TextSize, TextSnapshot, TokenFlags, reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c2), Revision(0), text).unwrap()
}

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2i_rule_for_test(source(text), rule, ParseConfig::default()).unwrap()
}

fn contains_kind(node: &SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected || node.children().any(|child| contains_kind(&child, expected))
}

fn count_kind(node: &SyntaxNode, expected: SyntaxKind) -> usize {
    usize::from(node.kind() == expected)
        + node
            .children()
            .map(|child| count_kind(&child, expected))
            .sum::<usize>()
}

fn nodes_of_kind(node: &SyntaxNode, expected: SyntaxKind) -> Vec<SyntaxNode> {
    let mut nodes = Vec::new();
    if node.kind() == expected {
        nodes.push(node.clone());
    }
    for child in node.children() {
        nodes.extend(nodes_of_kind(&child, expected));
    }
    nodes
}

fn assert_clean(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let parsed = parse(rule, text);
    assert_eq!(
        parsed.outcome,
        CanonicalRuleOutcome::Matched,
        "{rule:?} on {text:?}"
    );
    assert!(parsed.is_strictly_clean(), "{rule:?} on {text:?}");
    assert_eq!(parsed.consumed.end, TextSize(text.len() as u32));
    parsed
}

fn assert_transactional_nomatch(rule: RuleId, text: &str) {
    let parsed = parse(rule, text);
    assert_eq!(
        parsed.outcome,
        CanonicalRuleOutcome::NoMatch,
        "{rule:?} on {text:?}"
    );
    assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
    assert!(parsed.diagnostics.is_empty(), "{rule:?} on {text:?}");
    assert!(!parsed.root.flags.intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING
    ));
}

#[test]
fn precedence_nodes_are_conditional_and_nested_by_binding_strength() {
    let literal = assert_clean(rules::EXPRESSION, "1");
    for kind in [
        SyntaxKind::LogicExpression,
        SyntaxKind::ComparisonExpression,
        SyntaxKind::AdditiveExpression,
        SyntaxKind::MultiplicativeExpression,
        SyntaxKind::PowerExpression,
        SyntaxKind::TableExpression,
        SyntaxKind::SetExpression,
    ] {
        assert!(
            !contains_kind(&literal.syntax(), kind),
            "redundant {kind:?}"
        );
    }

    let expression = assert_clean(rules::EXPRESSION, "1 + 2 * 3");
    assert!(contains_kind(
        &expression.syntax(),
        SyntaxKind::AdditiveExpression
    ));
    assert!(contains_kind(
        &expression.syntax(),
        SyntaxKind::MultiplicativeExpression
    ));
    for (rule, text, kind) in [
        (rules::L1, "true && false", SyntaxKind::LogicExpression),
        (rules::L2, "1 == 2", SyntaxKind::ComparisonExpression),
        (rules::L3, "1 + 2", SyntaxKind::AdditiveExpression),
        (rules::L4, "2 * 3", SyntaxKind::MultiplicativeExpression),
        (rules::L5, "2 ^ 3", SyntaxKind::PowerExpression),
        (rules::L6, "a ⋈ b", SyntaxKind::TableExpression),
        (rules::L7, "{1} ∪ {2}", SyntaxKind::SetExpression),
    ] {
        let parsed = assert_clean(rule, text);
        assert!(contains_kind(&parsed.syntax(), kind), "{rule:?}");
    }
}

#[test]
fn comprehension_and_pattern_semantics_match_the_clean_language() {
    assert_clean(rules::MATRIX_COMPREHENSION, "[x | x <- xs]");
    assert_clean(rules::MATRIX_COMPREHENSION, "[x | y := 1]");
    assert_transactional_nomatch(rules::MATRIX_COMPREHENSION, "[x | true]");
    assert_clean(rules::SET_COMPREHENSION, "{x | true}");

    assert_clean(rules::PATTERN_ARRAY, "[head, ..., tail]");
    for invalid in [
        "[..., ..., tail]",
        "[head | middle | tail]",
        "[..., head | tail]",
        "[head |]",
        "[head | tail, extra]",
    ] {
        assert_transactional_nomatch(rules::PATTERN_ARRAY, invalid);
    }
}

#[test]
fn fsm_values_reject_matching_only_pattern_features_recursively() {
    for valid in [":value", ":value(payload)", "(a, b)", "[head, tail]"] {
        assert_clean(rules::FSM_VALUE, valid);
    }
    for invalid in ["*", "[head, ..., tail]", "[head | tail]", ":some(*)"] {
        assert_transactional_nomatch(rules::FSM_VALUE, invalid);
    }
}

#[test]
fn malformed_clean_candidates_remain_nondiagnostic() {
    for (rule, text) in [
        (rules::EXPRESSION, "x +"),
        (rules::EXPRESSION, "x =="),
        (rules::FUNCTION_CALL, "foo("),
        (rules::SLICE, "foo["),
        (rules::PARENTHETICAL_TERM, "(1"),
        (rules::MATRIX, "[1"),
        (rules::SET, "{1"),
        (rules::RANGE_EXPRESSION, "1.."),
        (rules::EXPRESSION, "x ?"),
    ] {
        assert_transactional_nomatch(rule, text);
    }

    let subscript = parse(rules::SUBSCRIPT, "[1][");
    assert_eq!(subscript.outcome, CanonicalRuleOutcome::Matched);
    assert_eq!(
        subscript.consumed,
        TextRange::new(TextSize::ZERO, TextSize(3))
    );
    assert!(subscript.diagnostics.is_empty());

    let fsm = parse(rules::FSM_INSTANCE, "#machine(");
    assert_eq!(fsm.outcome, CanonicalRuleOutcome::Matched);
    assert_eq!(fsm.consumed, TextRange::new(TextSize::ZERO, TextSize(8)));
    assert!(fsm.diagnostics.is_empty());
}

#[test]
fn required_list_cardinalities_and_definition_lookahead_are_enforced() {
    for (rule, text) in [
        (rules::MAP, "{}"),
        (rules::RECORD, "{}"),
        (rules::SET, "{}"),
        (rules::KIND_TABLE, "||"),
        (rules::KIND_RECORD, "{}"),
        (rules::KIND_TUPLE, "()"),
        (rules::INLINE_TABLE, "|a<u8>||"),
        (rules::REGULAR_TABLE, "|a<u8>|"),
    ] {
        assert_transactional_nomatch(rule, text);
    }
    assert_clean(rules::TUPLE, "()");
    assert_clean(rules::ARGUMENT_LIST, "()");
    assert_clean(rules::FSM_ARGS, "()");
    assert_transactional_nomatch(rules::VARIABLE_DEFINE, "x += 1");
    assert_transactional_nomatch(rules::VARIABLE_DEFINE, "x :=");
}

#[test]
fn incomplete_ranges_are_marker_safe_and_preserve_committed_children() {
    for text in ["1..", "1..="] {
        let parsed = std::panic::catch_unwind(|| parse(rules::EXPRESSION, text))
            .unwrap_or_else(|_| panic!("expression panicked on {text:?}"));
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{text:?}");
        assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
        assert!(parsed.diagnostics.is_empty());
    }

    let trailing = std::panic::catch_unwind(|| parse(rules::EXPRESSION, "1..10.."))
        .expect("optional trailing range operator must not panic");
    assert_eq!(trailing.outcome, CanonicalRuleOutcome::Matched);
    assert_eq!(trailing.consumed.end, TextSize(5));

    for rule in [rules::RANGE_EXPRESSION, rules::EXPRESSION] {
        let parsed = std::panic::catch_unwind(|| parse(rule, "\"unterminated"))
            .unwrap_or_else(|_| panic!("{rule:?} did not balance a committed literal"));
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        assert!(!parsed.diagnostics.is_empty());
        assert!(!contains_kind(
            &parsed.syntax(),
            SyntaxKind::RangeExpression
        ));
    }
}

#[test]
fn matrix_and_nonstandard_structure_shells_are_complete_and_reachable() {
    for text in ["[]", "[\n1\n]", "[──\n1\n]", "╭\n1\n╯", "╭1╯", "[|1]"] {
        let matrix = assert_clean(rules::MATRIX, text);
        assert!(contains_kind(&matrix.syntax(), SyntaxKind::Matrix));
        validate_lossless_range(&matrix.root, &matrix.source, matrix.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&matrix.root, &matrix.source, matrix.consumed).unwrap(),
            text
        );
        assert!(
            matrix
                .syntax()
                .tokens()
                .into_iter()
                .all(|token| !token.flags().contains(TokenFlags::SYNTHETIC))
        );
    }

    for (text, kind) in [("╭1╯", SyntaxKind::Matrix), ("|a: 1|", SyntaxKind::Record)] {
        for rule in [rules::STRUCTURE, rules::FACTOR, rules::EXPRESSION] {
            let parsed = assert_clean(rule, text);
            assert!(
                contains_kind(&parsed.syntax(), kind),
                "{rule:?} on {text:?}"
            );
        }
    }
}

#[test]
fn kind_and_inline_table_lists_follow_their_exact_cardinalities() {
    for text in ["|a|", "|a b<u8>|", "|a,b<u8>|"] {
        assert_clean(rules::KIND_TABLE, text);
    }
    assert_transactional_nomatch(rules::KIND_TABLE, "||");

    for (text, consumed) in [("[u8],1", 4), ("[u8]:,1", 5), ("[u8]:1,2", 8)] {
        let parsed = parse(rules::KIND_MATRIX, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
        assert_eq!(parsed.consumed.end, TextSize(consumed));
    }

    for text in ["1|", "1 2|", "  1   2  |"] {
        let row = assert_clean(rules::INLINE_TABLE_ROW, text);
        assert_eq!(
            count_kind(&row.syntax(), SyntaxKind::Expression),
            if text == "1|" { 1 } else { 2 }
        );
    }
    assert_clean(rules::INLINE_TABLE, "|a<u8> b<u8>|1 2|");
}

#[test]
fn record_candidates_fall_through_to_maps_after_complete_clean_failure() {
    let record = assert_clean(rules::EXPRESSION, "{a: 1, b: 2}");
    assert!(contains_kind(&record.syntax(), SyntaxKind::Record));
    assert!(!contains_kind(&record.syntax(), SyntaxKind::Map));
    assert_eq!(
        nodes_of_kind(&record.syntax(), SyntaxKind::RecordBinding).len(),
        2
    );

    for (text, entry_count, typed_key) in [
        ("{a: 1, 2: 3}", 2, false),
        ("{a<u8>: 1, 2: 3}", 2, true),
        ("{a: 1, b: 2, 3: 4}", 3, false),
    ] {
        for rule in [rules::STRUCTURE, rules::FACTOR, rules::EXPRESSION] {
            let map = assert_clean(rule, text);
            let syntax = map.syntax();
            assert!(contains_kind(&syntax, SyntaxKind::Map));
            assert!(!contains_kind(&syntax, SyntaxKind::Record));
            assert!(!contains_kind(&syntax, SyntaxKind::RecordBinding));

            let entries = nodes_of_kind(&syntax, SyntaxKind::MapEntry);
            assert_eq!(entries.len(), entry_count, "{rule:?} on {text:?}");
            for entry in &entries {
                assert_eq!(
                    entry
                        .children()
                        .filter(|child| child.kind() == SyntaxKind::Expression)
                        .count(),
                    2,
                    "{rule:?} on {text:?}"
                );
                assert!(!contains_kind(entry, SyntaxKind::RecordBinding));
            }

            let key = entries[0]
                .children()
                .find(|child| child.kind() == SyntaxKind::Expression)
                .expect("map entry must retain its key expression");
            let variables = nodes_of_kind(&key, SyntaxKind::Variable);
            assert_eq!(variables.len(), 1, "{rule:?} on {text:?}");
            let identifier = variables[0]
                .children()
                .find(|child| child.kind() == SyntaxKind::Identifier)
                .expect("variable must retain its direct identifier");
            assert_eq!(
                identifier
                    .tokens()
                    .into_iter()
                    .map(|token| token.text().expect("identifier token must be physical"))
                    .collect::<String>(),
                "a",
                "{rule:?} on {text:?}"
            );
            assert_eq!(
                contains_kind(&variables[0], SyntaxKind::KindAnnotation),
                typed_key,
                "{rule:?} on {text:?}"
            );
        }
    }
}

#[test]
fn set_comprehensions_cannot_become_formula_operands() {
    assert_transactional_nomatch(rules::EXPRESSION, "1 + {x | x <- xs}");
}

#[test]
fn optional_match_suffix_rewinds_unclaimed_whitespace() {
    let trailing = parse(rules::EXPRESSION, "x ");
    assert_eq!(trailing.outcome, CanonicalRuleOutcome::Matched);
    assert_eq!(trailing.consumed.end, TextSize(1));

    assert_transactional_nomatch(rules::EXPRESSION, "x ?");
}
