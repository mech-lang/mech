//! Owner, continuation, and diagnostic evidence for the nine S2 review findings.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DiagnosticPhase, DocumentId, ExpectedSyntax, NodeFlags, ParseConfig, RecoveryAction, Revision,
    RuleId, Severity, SyntaxElement, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot,
    TokenFlags, reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(0x52), Revision(7), text).unwrap();
    let parsed =
        parse_canonical_phase_2i_rule_for_test(source, rule, ParseConfig::default()).unwrap();
    assert_eq!(parsed.consumed, range(0, text.len()), "{rule:?}: {text:?}");
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        text,
    );
    parsed
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange::new(TextSize(start as u32), TextSize(end as u32))
}

fn nodes(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut result = Vec::new();
    if node.kind() == kind {
        result.push(node.clone());
    }
    for child in node.children() {
        result.extend(nodes(&child, kind));
    }
    result
}

fn only(node: &SyntaxNode, kind: SyntaxKind) -> SyntaxNode {
    let found = nodes(node, kind);
    assert_eq!(found.len(), 1, "expected one {kind:?} in {node:?}");
    found[0].clone()
}

fn direct(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    node.children()
        .filter(|child| child.kind() == kind)
        .collect()
}

fn texts(nodes: &[SyntaxNode]) -> Vec<String> {
    nodes.iter().map(|node| node.text().unwrap()).collect()
}

fn physical(node: &SyntaxNode, kind: SyntaxKind, text: &str, start: usize) {
    let token = node
        .children_with_tokens()
        .into_iter()
        .find_map(|element| match element {
            SyntaxElement::Token(token)
                if token.kind() == kind && token.range().start == TextSize(start as u32) =>
            {
                Some(token)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing direct {text:?} at {start} in {:?}", node.kind()));
    assert_eq!(token.text().unwrap(), text);
    assert_eq!(token.range(), range(start, start + text.len()));
    assert!(
        !token
            .flags()
            .intersects(TokenFlags::MISSING | TokenFlags::SYNTHETIC | TokenFlags::ERROR)
    );
}

fn inserted(
    parsed: &CanonicalSourceRuleSnapshot,
    code: &str,
    rule: RuleId,
    at: usize,
    expected: ExpectedSyntax,
    found_kind: SyntaxKind,
    found_text: &str,
) {
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert!(parsed.root.flags.contains(NodeFlags::CONTAINS_MISSING));
    assert!(nodes(&parsed.syntax(), SyntaxKind::Error).is_empty());
    let missing = only(&parsed.syntax(), SyntaxKind::Missing);
    assert_eq!(missing.range(), range(at, at));
    let diagnostic = parsed.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), code);
    assert_eq!(diagnostic.phase, DiagnosticPhase::Syntax);
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.rule, Some(rule));
    assert_eq!(diagnostic.context, None);
    assert_eq!(
        diagnostic
            .primary
            .resolve(parsed.source.revision(), &parsed.nodes),
        Some(range(at, at))
    );
    assert_eq!(diagnostic.expected, vec![expected.clone()]);
    let found = diagnostic.found.as_ref().unwrap();
    assert_eq!(found.kind, Some(found_kind));
    assert_eq!(found.text.as_deref(), Some(found_text));
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: expected,
            at: TextSize(at as u32)
        })
    );
}

fn expression() -> ExpectedSyntax {
    ExpectedSyntax::Production("expression".into())
}

// 3996399707
#[test]
fn recovered_pattern_array_element_keeps_later_sibling_and_owner_bracket() {
    for rule in [rules::PATTERN_ARRAY, rules::PATTERN] {
        let parsed = parse(rule, "[1, 2 +, 3]");
        let array = only(&parsed.syntax(), SyntaxKind::ArrayPattern);
        let items = direct(&array, SyntaxKind::ArrayPatternElement);
        assert_eq!(texts(&items), ["1", "2 +", "3"]);
        assert_eq!(only(&items[1], SyntaxKind::Missing).range(), range(7, 7));
        assert_eq!(
            only(&items[2], SyntaxKind::IntegerLiteral).text().unwrap(),
            "3"
        );
        physical(&array, SyntaxKind::LeftBracket, "[", 0);
        physical(&array, SyntaxKind::Comma, ",", 2);
        physical(&array, SyntaxKind::Comma, ",", 7);
        physical(&array, SyntaxKind::RightBracket, "]", 10);
        inserted(
            &parsed,
            "syntax/missing-operator-operand",
            rules::L3,
            7,
            expression(),
            SyntaxKind::Comma,
            ",",
        );
    }
}

// 3996399711
#[test]
fn recovered_map_value_keeps_later_mapping_in_direct_and_shared_paths() {
    for rule in [rules::MAP, rules::STRUCTURE, rules::EXPRESSION] {
        let parsed = parse(rule, "{1: 2, 3:, 4: 5}");
        let map = only(&parsed.syntax(), SyntaxKind::Map);
        assert!(nodes(&parsed.syntax(), SyntaxKind::Record).is_empty());
        let entries = direct(&map, SyntaxKind::MapEntry);
        assert_eq!(texts(&entries), ["1: 2, ", "3:", "4: 5"]);
        assert_eq!(only(&entries[1], SyntaxKind::Missing).range(), range(9, 9));
        assert_eq!(
            texts(&direct(&entries[2], SyntaxKind::Expression)),
            ["4", "5"]
        );
        physical(&map, SyntaxKind::LeftBrace, "{", 0);
        physical(&map, SyntaxKind::Comma, ",", 9);
        physical(&map, SyntaxKind::RightBrace, "}", 15);
        inserted(
            &parsed,
            "syntax/missing-mapping-value",
            rules::MAPPING,
            9,
            expression(),
            SyntaxKind::Comma,
            ",",
        );
    }
}

// 3996399715
#[test]
fn recovered_match_pattern_keeps_arm_output_operator_and_result() {
    for (rule, text, shift) in [
        (rules::MATCH_ARM, "| (1 +) => 2", 0),
        (rules::EXPRESSION, "x ? | (1 +) => 2", 4),
    ] {
        let parsed = parse(rule, text);
        let arm = only(&parsed.syntax(), SyntaxKind::MatchArm);
        assert_eq!(arm.text().unwrap(), "| (1 +) => 2");
        let pattern = direct(&arm, SyntaxKind::Pattern);
        assert_eq!(texts(&pattern), ["(1 +)"]);
        assert_eq!(texts(&direct(&arm, SyntaxKind::Expression)), ["2"]);
        let tuple = only(&pattern[0], SyntaxKind::TuplePattern);
        physical(&tuple, SyntaxKind::LeftParen, "(", shift + 2);
        physical(&tuple, SyntaxKind::RightParen, ")", shift + 6);
        physical(&arm, SyntaxKind::OutputOperator, "=>", shift + 8);
        inserted(
            &parsed,
            "syntax/missing-operator-operand",
            rules::L3,
            shift + 6,
            expression(),
            SyntaxKind::RightParen,
            ")",
        );
    }
}

// 3996399719
#[test]
fn missing_tuple_item_keeps_ordered_siblings_and_physical_closer() {
    for rule in [rules::TUPLE, rules::STRUCTURE, rules::EXPRESSION] {
        let parsed = parse(rule, "(1,,3)");
        let tuple = only(&parsed.syntax(), SyntaxKind::Tuple);
        assert!(nodes(&parsed.syntax(), SyntaxKind::ParentheticalExpression).is_empty());
        assert_eq!(
            tuple
                .children()
                .map(|child| child.kind())
                .collect::<Vec<_>>(),
            [
                SyntaxKind::Expression,
                SyntaxKind::Missing,
                SyntaxKind::Expression
            ]
        );
        assert_eq!(texts(&direct(&tuple, SyntaxKind::Expression)), ["1", "3"]);
        physical(&tuple, SyntaxKind::LeftParen, "(", 0);
        physical(&tuple, SyntaxKind::Comma, ",", 2);
        physical(&tuple, SyntaxKind::Comma, ",", 3);
        physical(&tuple, SyntaxKind::RightParen, ")", 5);
        inserted(
            &parsed,
            "syntax/missing-tuple-item",
            if rule == rules::EXPRESSION {
                rules::FACTOR
            } else {
                rules::TUPLE
            },
            3,
            expression(),
            SyntaxKind::Comma,
            ",",
        );
    }
}

// 3996399720 and the corresponding shared-bracket continuation.
#[test]
fn recovered_comprehension_head_keeps_selected_form_and_qualifier() {
    for (rule, text, kind, open, close) in [
        (
            rules::SET_COMPREHENSION,
            "{1 + | x <- xs}",
            SyntaxKind::SetComprehension,
            SyntaxKind::LeftBrace,
            SyntaxKind::RightBrace,
        ),
        (
            rules::EXPRESSION,
            "{1 + | x <- xs}",
            SyntaxKind::SetComprehension,
            SyntaxKind::LeftBrace,
            SyntaxKind::RightBrace,
        ),
        (
            rules::MATRIX_COMPREHENSION,
            "[1 + | x <- xs]",
            SyntaxKind::MatrixComprehension,
            SyntaxKind::LeftBracket,
            SyntaxKind::RightBracket,
        ),
        (
            rules::EXPRESSION,
            "[1 + | x <- xs]",
            SyntaxKind::MatrixComprehension,
            SyntaxKind::LeftBracket,
            SyntaxKind::RightBracket,
        ),
    ] {
        let parsed = parse(rule, text);
        let owner = only(&parsed.syntax(), kind);
        assert_eq!(owner.text().unwrap(), text);
        assert!(nodes(&parsed.syntax(), SyntaxKind::Set).is_empty());
        assert!(nodes(&parsed.syntax(), SyntaxKind::Matrix).is_empty());
        assert_eq!(texts(&direct(&owner, SyntaxKind::Expression)), ["1 + "]);
        let qualifiers = direct(&owner, SyntaxKind::ComprehensionQualifier);
        assert_eq!(texts(&qualifiers), ["x <- xs"]);
        let generator = only(&qualifiers[0], SyntaxKind::Generator);
        assert_eq!(texts(&direct(&generator, SyntaxKind::Pattern)), ["x"]);
        assert_eq!(texts(&direct(&generator, SyntaxKind::Expression)), ["xs"]);
        physical(&owner, open, &text[..1], 0);
        physical(&owner, SyntaxKind::Bar, "|", 5);
        physical(&owner, close, &text[14..], 14);
        inserted(
            &parsed,
            "syntax/missing-operator-operand",
            rules::L3,
            5,
            expression(),
            SyntaxKind::Bar,
            "|",
        );
    }
}

// 3996399722
#[test]
fn recovered_fsm_instance_keeps_valid_pipe_stage() {
    for rule in [rules::FSM_PIPE, rules::EXPRESSION] {
        let parsed = parse(rule, "# -> :next");
        let pipe = only(&parsed.syntax(), SyntaxKind::FsmPipe);
        assert_eq!(
            pipe.children()
                .map(|child| child.kind())
                .collect::<Vec<_>>(),
            [SyntaxKind::FsmInstance, SyntaxKind::FsmStateTransition]
        );
        let instance = only(&pipe, SyntaxKind::FsmInstance);
        assert!(nodes(&instance, SyntaxKind::Identifier).is_empty());
        assert_eq!(only(&instance, SyntaxKind::Missing).range(), range(2, 2));
        physical(&instance, SyntaxKind::HashTag, "#", 0);
        let stage = only(&pipe, SyntaxKind::FsmStateTransition);
        physical(&stage, SyntaxKind::TransitionOperator, "->", 2);
        assert_eq!(texts(&direct(&stage, SyntaxKind::FsmValue)), [":next"]);
        inserted(
            &parsed,
            "syntax/missing-fsm-name",
            rules::FSM_INSTANCE,
            2,
            ExpectedSyntax::Production("identifier".into()),
            SyntaxKind::TransitionOperator,
            "->",
        );
    }
}

// 3996399725
#[test]
fn recovered_framed_header_keeps_data_row_and_each_vertical_delimiter() {
    for rule in [rules::FANCY_TABLE, rules::TABLE, rules::EXPRESSION] {
        let parsed = parse(rule, "╭─\n│a<u8│\n│1│");
        let table = only(&parsed.syntax(), SyntaxKind::FancyTable);
        assert!(nodes(&parsed.syntax(), SyntaxKind::Matrix).is_empty());
        let header = only(&table, SyntaxKind::FancyTableHeader);
        assert_eq!(texts(&direct(&header, SyntaxKind::TableField)), ["a<u8"]);
        let row = only(&table, SyntaxKind::FancyTableRow);
        assert_eq!(texts(&direct(&row, SyntaxKind::Expression)), ["1"]);
        physical(&table, SyntaxKind::BoxDrawing, "╭", 0);
        physical(&table, SyntaxKind::BoxDrawing, "│", 7);
        physical(&header, SyntaxKind::BoxDrawing, "│", 14);
        physical(&row, SyntaxKind::BoxDrawing, "│", 18);
        physical(&row, SyntaxKind::BoxDrawing, "│", 22);
        let annotation = only(&header, SyntaxKind::KindAnnotation);
        let missing = only(&annotation, SyntaxKind::Missing);
        let synthetic = missing.tokens();
        assert_eq!(synthetic.len(), 1);
        assert_eq!(synthetic[0].kind(), SyntaxKind::RightAngle);
        assert!(
            synthetic[0]
                .flags()
                .contains(TokenFlags::MISSING | TokenFlags::SYNTHETIC)
        );
        inserted(
            &parsed,
            "syntax/missing-delimiter",
            rules::KIND_ANNOTATION,
            14,
            ExpectedSyntax::Token(SyntaxKind::RightAngle),
            SyntaxKind::BoxDrawing,
            "│",
        );
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.fixes.len(), 1);
        assert_eq!(diagnostic.fixes[0].edits[0].delete, range(14, 14));
        assert_eq!(diagnostic.fixes[0].edits[0].insert, ">");
    }
}

// 3996399727
#[test]
fn delimited_mapping_key_selects_clean_map_after_record_prefix() {
    for rule in [rules::MAP, rules::STRUCTURE, rules::EXPRESSION] {
        for key in ["(2)", "[2]", "{2}"] {
            let text = format!("{{a: 1, {key}: 3}}");
            let parsed = parse(rule, &text);
            assert!(parsed.is_strictly_clean());
            let map = only(&parsed.syntax(), SyntaxKind::Map);
            assert!(nodes(&parsed.syntax(), SyntaxKind::Record).is_empty());
            let entries = direct(&map, SyntaxKind::MapEntry);
            assert_eq!(entries.len(), 2);
            assert_eq!(
                texts(&direct(&entries[0], SyntaxKind::Expression)),
                ["a", "1"]
            );
            assert_eq!(
                texts(&direct(&entries[1], SyntaxKind::Expression)),
                [key, "3"]
            );
            physical(&entries[1], SyntaxKind::Colon, ":", 10);
            physical(&map, SyntaxKind::LeftBrace, "{", 0);
            physical(&map, SyntaxKind::RightBrace, "}", 13);
        }
    }
}

// 3996399728
#[test]
fn framed_owner_closer_stays_outside_error_behind_unmatched_nested_opener() {
    for rule in [rules::MATRIX, rules::EXPRESSION] {
        for (open, close) in [("╭", "╯"), ("┌", "┘"), ("┏", "┛")] {
            let text = format!("{open}1 @ (2{close}");
            let parsed = parse(rule, &text);
            assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
            let matrix = only(&parsed.syntax(), SyntaxKind::Matrix);
            assert_eq!(
                only(&matrix, SyntaxKind::IntegerLiteral).text().unwrap(),
                "1"
            );
            let error = only(&matrix, SyntaxKind::Error);
            assert_eq!(error.text().unwrap(), "@ (2");
            assert_eq!(error.range(), range(5, 9));
            assert!(nodes(&matrix, SyntaxKind::Missing).is_empty());
            physical(&matrix, SyntaxKind::BoxDrawing, open, 0);
            physical(&matrix, SyntaxKind::BoxDrawing, close, 9);
            assert_eq!(parsed.diagnostics.len(), 1);
            let diagnostic = parsed.diagnostics.iter().next().unwrap();
            assert_eq!(
                diagnostic.code.as_str(),
                "syntax/unexpected-delimited-content"
            );
            assert_eq!(diagnostic.phase, DiagnosticPhase::Syntax);
            assert_eq!(diagnostic.severity, Severity::Error);
            assert_eq!(diagnostic.rule, Some(rules::MATRIX));
            assert_eq!(diagnostic.context, None);
            assert_eq!(
                diagnostic
                    .primary
                    .resolve(parsed.source.revision(), &parsed.nodes),
                Some(range(5, 9))
            );
            assert!(diagnostic.expected.is_empty());
            assert_eq!(
                diagnostic.found.as_ref().unwrap().kind,
                Some(SyntaxKind::Unknown)
            );
            assert_eq!(
                diagnostic.found.as_ref().unwrap().text.as_deref(),
                Some("@ (2")
            );
            assert_eq!(
                diagnostic.recovery,
                Some(RecoveryAction::Abandon {
                    rule: rules::MATRIX,
                    at: TextSize(9)
                })
            );
        }
    }
}

#[test]
fn range_dots_after_a_variable_are_not_dispatched_as_a_slice() {
    for rule in [rules::RANGE_EXPRESSION, rules::EXPRESSION] {
        for text in [
            "limit..10",
            "limit..=10",
            "0..limit",
            "0..=limit",
            "limit..1..10",
            "0..limit..10",
            "0..1..limit",
            "0..limit..=10",
        ] {
            let parsed = parse(rule, text);
            assert!(parsed.is_strictly_clean(), "{rule:?} {text:?}");
            let range = only(&parsed.syntax(), SyntaxKind::RangeExpression);
            assert!(nodes(&range, SyntaxKind::Slice).is_empty());
            let operators = direct(&range, SyntaxKind::RangeOperator);
            assert_eq!(operators.len(), text.matches("..").count());
            let bounds = direct(&range, SyntaxKind::Factor);
            assert_eq!(bounds.len(), operators.len() + 1);
            assert_eq!(only(&range, SyntaxKind::Variable).text().unwrap(), "limit");
        }
    }
    for text in ["limit.field", "limit.1", "limit[1]"] {
        let parsed = parse(rules::EXPRESSION, text);
        assert!(parsed.is_strictly_clean());
        assert_eq!(
            only(&parsed.syntax(), SyntaxKind::Slice).text().unwrap(),
            text
        );
    }
}
