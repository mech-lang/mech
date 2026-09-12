use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::{canonical_rule_name, rules};
use mech_syntax::document::{
    DocumentId, ExpectedSyntax, NodeFlags, ParseConfig, ParseLimits, RecoveryAction, Revision,
    RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, reconstruct_source_range,
    validate_lossless_range,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const MALFORMED_CASES: &[(RuleId, &str)] = &[
    (rules::ARGUMENT_LIST, "(1"),
    (rules::BINDING, "a:"),
    (rules::BRACE_SUBSCRIPT, "{1"),
    (rules::BRACKET_SUBSCRIPT, "[1"),
    (rules::CALL_ARG, "\"unterminated"),
    (rules::CALL_ARG_WITH_BINDING, "x:"),
    (rules::COMPREHENSION_QUALIFIER, "x <-"),
    (rules::EXPRESSION, "1 +"),
    (rules::FACTOR, "\"unterminated"),
    (rules::FANCY_TABLE, "╭─\n│a│"),
    (rules::FANCY_TABLE_HEADER, "a"),
    (rules::FIELD, "a<u8"),
    (rules::FORMULA, "1 +"),
    (rules::FORMULA_SUBSCRIPT, "1 +"),
    (rules::FSM_ARGS, "(1"),
    (rules::FSM_ASYNC_TRANSITION, "~>"),
    (rules::FSM_INSTANCE, "#machine("),
    (rules::FSM_OUTPUT, "=>"),
    (rules::FSM_PIPE, "#machine ->"),
    (rules::FSM_STATE_TRANSITION, "->"),
    (rules::FSM_VALUE, "\"unterminated"),
    (rules::FUNCTION_CALL, "foo("),
    (rules::GENERATOR, "x <-"),
    (rules::HEADER_FIELD, "a<u8"),
    (rules::INLINE_TABLE, "|a<u8>|"),
    (rules::INLINE_TABLE_HEADER, "a<u8>"),
    (rules::INLINE_TABLE_ROW, "1"),
    (rules::KIND, "<u8"),
    (rules::KIND_ANNOTATION, "<u8"),
    (rules::KIND_KIND, "<u8"),
    (rules::KIND_MAP, "{u8:"),
    (rules::KIND_MATRIX, "[u8"),
    (rules::KIND_RECORD, "{a<u8>"),
    (rules::KIND_SCALAR, "u8:1.."),
    (rules::KIND_SET, "{u8"),
    (rules::KIND_TABLE, "|a<u8>"),
    (rules::KIND_TUPLE, "(u8"),
    (rules::KIND_WITH_OPTION, "<u8"),
    (rules::L1, "true &&"),
    (rules::L2, "1 =="),
    (rules::L3, "1 +"),
    (rules::L4, "2 *"),
    (rules::L5, "2 ^"),
    (rules::L6, "a ⋈"),
    (rules::L7, "{1} ∪"),
    (rules::LITERAL, "\"unterminated"),
    (rules::MAP, "{1:"),
    (rules::MAPPING, "1:"),
    (rules::MATCH_ARM, "| * =>"),
    (rules::MATRIX, "[1"),
    (rules::MATRIX_COLUMN, "\"unterminated"),
    (rules::MATRIX_COMPREHENSION, "[x | x <-"),
    (rules::MATRIX_ROW, "\"unterminated"),
    (rules::NEGATE_FACTOR, "-"),
    (rules::NOT_FACTOR, "!"),
    (rules::PARENTHETICAL_TERM, "(1"),
    (rules::PATTERN, "\"unterminated"),
    (rules::PATTERN_ARRAY, "[head"),
    (rules::PATTERN_ARRAY_ITEM, "\"unterminated"),
    (rules::PATTERN_ARRAY_TOKEN, "\"unterminated"),
    (rules::PATTERN_ATOM_STRUCT, ":some(\"unterminated"),
    (rules::PATTERN_TUPLE, "(\"unterminated"),
    (rules::PATTERN_TUPLE_STRUCT, "`some(\"unterminated"),
    (rules::RANGE_EXPRESSION, "1.."),
    (rules::RANGE_SUBSCRIPT, "1.."),
    (rules::RECORD, "{a:"),
    (rules::REGULAR_TABLE, "|a<u8>|"),
    (rules::SET, "{1"),
    (rules::SET_COMPREHENSION, "{x | x <-"),
    (rules::SLICE, "x[1"),
    (rules::STRUCTURE, "(1"),
    (rules::SUBSCRIPT, "[1"),
    (rules::TABLE, "|a<u8>|"),
    (rules::TABLE_HEADER, "a<u8>"),
    (rules::TABLE_ROW, "|1"),
    (rules::TABLE_ROW2, "|1"),
    (rules::TUPLE, "(1"),
    (rules::TUPLE_STRUCT, ":some(1"),
    (rules::VAR, "x<u8"),
    (rules::VARIABLE_DEFINE, "x :="),
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c6), Revision(0), text).unwrap()
}

fn parse(
    rule: RuleId,
    text: &str,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
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

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn syntax_schema() -> BTreeMap<String, (String, String)> {
    fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .expect("read phase-2i-syntax-schema.tsv")
    .lines()
    .skip(1)
    .map(|line| {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6);
        (
            fields[0].to_owned(),
            (fields[2].to_owned(), fields[3].to_owned()),
        )
    })
    .collect()
}

fn contains_named_kind(node: &SyntaxNode, expected: &str) -> bool {
    format!("{:?}", node.kind()) == expected
        || node
            .children()
            .any(|child| contains_named_kind(&child, expected))
}

#[test]
fn every_phase_2i_rule_retains_a_lossless_malformed_direct_root() {
    assert_eq!(MALFORMED_CASES.len(), 80);
    let schema = syntax_schema();
    let mut failures = Vec::new();
    for (rule, text) in MALFORMED_CASES {
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source(text), *rule, ParseConfig::default())
                .unwrap_or_else(|| panic!("missing Phase 2I dispatcher arm for {rule:?}"));
        let expected_range = TextRange::new(TextSize::ZERO, TextSize(text.len() as u32));
        if parsed.outcome != CanonicalRuleOutcome::Committed
            || parsed.consumed != expected_range
            || parsed.diagnostics.is_empty()
            || !parsed.root.flags.intersects(
                NodeFlags::ERROR
                    | NodeFlags::MISSING
                    | NodeFlags::CONTAINS_ERROR
                    | NodeFlags::CONTAINS_MISSING,
            )
        {
            failures.push(format!(
                "{} ({rule:?}) on {text:?}: outcome={:?}, consumed={:?}, diagnostics={}",
                canonical_rule_name(*rule).unwrap_or("unknown"),
                parsed.outcome,
                parsed.consumed,
                parsed.diagnostics.len()
            ));
            continue;
        }
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
            .unwrap_or_else(|error| panic!("{rule:?} on {text:?} was not lossless: {error:?}"));
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            *text,
            "{rule:?} on {text:?}"
        );
        let name = canonical_rule_name(*rule).unwrap();
        let (policy, kind) = &schema[name];
        let partial_kind = match policy.as_str() {
            "node" | "conditional-node" => kind.as_str(),
            "transparent" if name == "formula" => "AdditiveExpression",
            "transparent" if name == "pattern-array-item" => "Pattern",
            other => panic!("unknown recovery schema policy {other:?} for {name}"),
        };
        assert!(
            contains_named_kind(&parsed.syntax(), partial_kind),
            "{name} on {text:?} did not retain partial {partial_kind}"
        );
        for diagnostic in parsed.diagnostics.iter() {
            assert!(diagnostic.rule.is_some(), "{rule:?} on {text:?}");
            let action = diagnostic
                .recovery
                .as_ref()
                .unwrap_or_else(|| panic!("missing recovery action for {rule:?} on {text:?}"));
            match action {
                RecoveryAction::Insert { at, .. } | RecoveryAction::Abandon { at, .. } => {
                    assert!(*at <= parsed.consumed.end, "{rule:?} on {text:?}");
                }
                RecoveryAction::Skip { range } | RecoveryAction::ResourceLimit { range } => {
                    assert!(range.end <= parsed.consumed.end, "{rule:?} on {text:?}");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn absent_and_unexpected_operands_have_distinct_recovery_actions() {
    let absent = parse(rules::EXPRESSION, "1 +");
    assert_eq!(absent.outcome, CanonicalRuleOutcome::Committed);
    assert!(contains_kind(&absent.syntax(), SyntaxKind::Missing));
    assert!(!contains_kind(&absent.syntax(), SyntaxKind::Error));
    let diagnostic = absent.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/missing-operator-operand");
    assert_eq!(diagnostic.rule, Some(rules::L3));
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: ExpectedSyntax::Production("expression".into()),
            at: TextSize(3),
        })
    );

    let unexpected = parse(rules::EXPRESSION, "1 + @");
    assert_eq!(unexpected.outcome, CanonicalRuleOutcome::Committed);
    assert!(contains_kind(&unexpected.syntax(), SyntaxKind::Error));
    assert!(!contains_kind(&unexpected.syntax(), SyntaxKind::Missing));
    let diagnostic = unexpected.diagnostics.iter().next().unwrap();
    assert_eq!(
        diagnostic.code.as_str(),
        "syntax/unexpected-production-source"
    );
    assert_eq!(diagnostic.rule, Some(rules::L3));
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Abandon {
            rule: rules::L3,
            at: TextSize(5),
        })
    );
}

#[test]
fn delimiter_recovery_skips_nested_source_and_leaves_ancestor_closers() {
    let nested = parse(rules::ARGUMENT_LIST, "(1 @ [2])");
    assert_eq!(nested.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(nested.consumed.end, TextSize(9));
    assert!(contains_kind(&nested.syntax(), SyntaxKind::Error));
    assert!(!contains_kind(&nested.syntax(), SyntaxKind::Missing));
    let diagnostic = nested.diagnostics.iter().next().unwrap();
    assert_eq!(
        diagnostic.code.as_str(),
        "syntax/unexpected-delimited-content"
    );
    assert_eq!(diagnostic.rule, Some(rules::ARGUMENT_LIST));
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Abandon {
            rule: rules::ARGUMENT_LIST,
            at: TextSize(8),
        })
    );

    let ancestor = parse(rules::BRACKET_SUBSCRIPT, "[1)");
    assert_eq!(ancestor.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(ancestor.consumed.end, TextSize(2));
    assert!(!contains_kind(&ancestor.syntax(), SyntaxKind::Error));
    assert!(contains_kind(&ancestor.syntax(), SyntaxKind::Missing));
    let diagnostic = ancestor.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/missing-delimiter");
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: ExpectedSyntax::Token(SyntaxKind::RightBracket),
            at: TextSize(2),
        })
    );
}

#[test]
fn mismatched_nested_closer_restarts_the_owning_delimited_rule() {
    let parsed = parse(rules::ARGUMENT_LIST, "(1 @ [2)");
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end, TextSize(8));
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Error));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Missing));
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        "(1 @ [2)"
    );
    assert_eq!(
        parsed.diagnostics.iter().next().unwrap().recovery,
        Some(RecoveryAction::Abandon {
            rule: rules::ARGUMENT_LIST,
            at: TextSize(7),
        })
    );
}

#[test]
fn angle_closer_remains_physical_when_the_required_kind_is_absent() {
    for text in ["<>", "<⟩"] {
        let parsed = parse(rules::KIND_ANNOTATION, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text:?}");
        assert!(
            contains_kind(&parsed.syntax(), SyntaxKind::Missing),
            "{text:?}"
        );
        assert!(
            !contains_kind(&parsed.syntax(), SyntaxKind::Error),
            "{text:?}"
        );
        let right_angles = parsed
            .syntax()
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::RightAngle)
            .collect::<Vec<_>>();
        assert_eq!(right_angles.len(), 1, "{text:?}");
        assert!(
            !right_angles[0]
                .flags()
                .contains(mech_syntax::document::TokenFlags::MISSING)
        );
    }
}

#[test]
fn committed_kind_record_recovery_keeps_its_selected_form_exclusive() {
    let parsed = parse(rules::KIND, "{a<u8");
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::KindRecord));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::KindSet));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::KindMap));
}

#[test]
fn record_tail_recovery_preserves_a_selected_direct_record() {
    let text = "{a: 1 @}";
    let parsed = parse(rules::RECORD, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Record));
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Error));
    assert_eq!(
        parsed.diagnostics.iter().next().unwrap().code.as_str(),
        "syntax/unexpected-delimited-content"
    );

    let map = parse(rules::RECORD, "{a: 1, 2: 3}");
    assert_eq!(map.outcome, CanonicalRuleOutcome::NoMatch);
    assert_eq!(map.consumed, TextRange::empty(TextSize::ZERO));
}

#[test]
fn tuple_recovery_resumes_at_the_next_sibling_separator() {
    for rule in [rules::TUPLE, rules::EXPRESSION] {
        let text = "(1, 2 +, 3)";
        let parsed = parse(rule, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{rule:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{rule:?}");
        let expected_expressions = if rule == rules::TUPLE { 3 } else { 4 };
        assert_eq!(
            count_kind(&parsed.syntax(), SyntaxKind::Expression),
            expected_expressions,
            "{rule:?}"
        );
        assert!(
            contains_kind(&parsed.syntax(), SyntaxKind::Missing),
            "{rule:?}"
        );
        assert!(
            !contains_kind(&parsed.syntax(), SyntaxKind::Error),
            "{rule:?}"
        );
    }
}

#[test]
fn call_recovery_resumes_at_the_next_sibling_separator() {
    let text = "(1, 2 +, 3)";
    let parsed = parse(rules::ARGUMENT_LIST, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::CallArgument), 3);
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Missing));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Error));
}

#[test]
fn recursive_list_recovery_resumes_at_later_siblings() {
    for (rule, text, kind, expected) in [
        (
            rules::KIND_TUPLE,
            "(u8, <u8, i8)",
            SyntaxKind::KindScalar,
            3,
        ),
        (
            rules::BRACKET_SUBSCRIPT,
            "[1, 2 +, 3]",
            SyntaxKind::IntegerLiteral,
            3,
        ),
        (
            rules::PATTERN_TUPLE,
            "(1, 2 +, 3)",
            SyntaxKind::IntegerLiteral,
            3,
        ),
        (
            rules::SET_COMPREHENSION,
            "{x | x <- xs, y <-, z <- zs}",
            SyntaxKind::Generator,
            3,
        ),
    ] {
        let parsed = parse(rule, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{rule:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{rule:?}");
        assert_eq!(count_kind(&parsed.syntax(), kind), expected, "{rule:?}");
        assert!(
            contains_kind(&parsed.syntax(), SyntaxKind::Missing),
            "{rule:?}"
        );
    }
}

#[test]
fn direct_collection_recovery_retains_later_siblings() {
    for (rule, text, kind, expected) in [
        (rules::SET, "{1, 2 +, 3}", SyntaxKind::IntegerLiteral, 3),
        (rules::MAP, "{1: 2, 3:, 4: 5}", SyntaxKind::MapEntry, 3),
        (
            rules::RECORD,
            "{a: 1, b:, c: 3}",
            SyntaxKind::RecordBinding,
            3,
        ),
        (rules::TUPLE, "(1,,3)", SyntaxKind::IntegerLiteral, 2),
        (rules::KIND_TABLE, "|a,,c|", SyntaxKind::Identifier, 2),
    ] {
        let parsed = parse(rule, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text:?}");
        assert_eq!(count_kind(&parsed.syntax(), kind), expected, "{text:?}");
        assert!(
            contains_kind(&parsed.syntax(), SyntaxKind::Missing),
            "{text:?}"
        );
    }
}

#[test]
fn recovered_table_cells_preserve_physical_row_boundaries() {
    let regular = "|a<u8>|\n|1 +\n|2|";
    let parsed = parse(rules::REGULAR_TABLE, regular);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, regular.len());
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::TableRow), 2);
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Missing));

    let framed = "│1 +│";
    let parsed = parse(rules::TABLE_ROW2, framed);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, framed.len());
    assert_eq!(
        parsed
            .syntax()
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::BoxDrawing)
            .count(),
        2
    );
}

#[test]
fn recovered_matrix_row_retains_later_rows() {
    let text = "[1; 2 +; 3]";
    let parsed = parse(rules::MATRIX, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::IntegerLiteral), 3);
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::MatrixRow), 3);
}

#[test]
fn recovered_kind_owners_consume_their_physical_braces() {
    for (text, selected) in [
        ("{u8:<u8}", SyntaxKind::KindMap),
        ("{a<u8>, b<u8}", SyntaxKind::KindRecord),
    ] {
        let parsed = parse(rules::KIND, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text:?}");
        assert!(contains_kind(&parsed.syntax(), selected), "{text:?}");
        let right_braces = parsed
            .syntax()
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::RightBrace)
            .collect::<Vec<_>>();
        assert_eq!(right_braces.len(), 1, "{text:?}");
        assert!(
            !right_braces[0]
                .flags()
                .contains(mech_syntax::document::TokenFlags::MISSING),
            "{text:?}"
        );
    }
}

#[test]
fn match_and_fsm_recovery_resume_at_later_stages() {
    let match_text = "x ? | * => 1 | * => | * => 3";
    let parsed = parse(rules::EXPRESSION, match_text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, match_text.len());
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::MatchArm), 3);

    let fsm_text = "#m -> => :next";
    let parsed = parse(rules::FSM_PIPE, fsm_text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, fsm_text.len());
    assert_eq!(
        count_kind(&parsed.syntax(), SyntaxKind::FsmStateTransition),
        1
    );
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::FsmOutput), 1);
}

#[test]
fn shared_set_recovery_resumes_at_a_later_item() {
    let text = "{1, 2 +, 3}";
    let parsed = parse(rules::EXPRESSION, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::IntegerLiteral), 3);
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Set));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Error));
}

#[test]
fn shared_brace_recovery_keeps_map_and_set_selection_exclusive() {
    for (text, selected, rejected) in [
        ("{1, 2 +}", SyntaxKind::Set, SyntaxKind::Map),
        ("{1: 2, 3:}", SyntaxKind::Map, SyntaxKind::Set),
    ] {
        let parsed = parse(rules::EXPRESSION, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text:?}");
        assert!(contains_kind(&parsed.syntax(), selected), "{text:?}");
        assert!(!contains_kind(&parsed.syntax(), rejected), "{text:?}");
        assert!(
            contains_kind(&parsed.syntax(), SyntaxKind::Missing),
            "{text:?}"
        );
    }
}

#[test]
fn recovered_parenthetical_selects_and_closes_one_owner() {
    let text = "(1 +)";
    let parsed = parse(rules::EXPRESSION, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(
        &parsed.syntax(),
        SyntaxKind::ParentheticalExpression
    ));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Tuple));
}

#[test]
fn converted_shared_map_finishes_its_physical_closer() {
    let text = "{a: 1, 2:}";
    let parsed = parse(rules::EXPRESSION, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Map));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Record));
}

#[test]
fn raw_triple_string_colons_do_not_select_a_shared_map() {
    let text = "{a: 1, \"\"\"a\":b\"\"\"}";
    let parsed = parse(rules::RECORD, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Record));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Map));
}

#[test]
fn committed_matrix_row_finishes_its_physical_closer() {
    let text = "[1; 2 +]";
    let parsed = parse(rules::MATRIX, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Matrix));
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Missing));
}

#[test]
fn first_item_recovery_finishes_one_shared_brace_owner() {
    let set = parse(rules::EXPRESSION, "{1 +}");
    assert_eq!(set.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(set.consumed.end.0 as usize, "{1 +}".len());
    assert!(contains_kind(&set.syntax(), SyntaxKind::Set));
    for rejected in [
        SyntaxKind::Map,
        SyntaxKind::Record,
        SyntaxKind::SetComprehension,
    ] {
        assert!(!contains_kind(&set.syntax(), rejected));
    }

    let kind_set = parse(rules::KIND, "{<u8}");
    assert_eq!(kind_set.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(kind_set.consumed.end.0 as usize, "{<u8}".len());
    assert!(contains_kind(&kind_set.syntax(), SyntaxKind::KindSet));
    assert!(!contains_kind(&kind_set.syntax(), SyntaxKind::KindMap));
}

#[test]
fn committed_record_children_finish_the_physical_owner_brace() {
    for (rule, text, kind) in [
        (rules::EXPRESSION, "{a: 1, b:}", SyntaxKind::Record),
        (rules::KIND, "{a<u8}", SyntaxKind::KindRecord),
    ] {
        let parsed = parse(rule, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text:?}");
        assert!(contains_kind(&parsed.syntax(), kind), "{text:?}");
        let closers = parsed
            .syntax()
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::RightBrace)
            .collect::<Vec<_>>();
        assert_eq!(closers.len(), 1, "{text:?}");
        assert!(
            !closers[0]
                .flags()
                .contains(mech_syntax::document::TokenFlags::MISSING),
            "{text:?}"
        );
    }
}

#[test]
fn table_recovery_preserves_the_regular_owner_and_following_rows() {
    for (rule, text, expected_rows) in [
        (rules::TABLE, "|a<u8>\n|1|", 1),
        (rules::REGULAR_TABLE, "|a<u8>\n|1|", 1),
        (rules::TABLE, "|a<u8>|\n|\n|2|", 2),
    ] {
        let parsed = parse(rule, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text:?}");
        assert!(
            contains_kind(&parsed.syntax(), SyntaxKind::RegularTable),
            "{text:?}"
        );
        assert!(
            !contains_kind(&parsed.syntax(), SyntaxKind::InlineTable),
            "{text:?}"
        );
        assert_eq!(
            count_kind(&parsed.syntax(), SyntaxKind::TableRow),
            expected_rows,
            "{text:?}"
        );
    }
}

#[test]
fn recovery_distinguishes_raw_triples_atoms_and_empty_call_arguments() {
    let raw = r#"(1 @ """abc\""")"#;
    let parsed = parse(rules::ARGUMENT_LIST, raw);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, raw.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Error));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Missing));

    let record = parse(rules::RECORD, "{a: 1, :foo}");
    assert_eq!(record.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(record.consumed.end.0 as usize, "{a: 1, :foo}".len());
    assert!(contains_kind(&record.syntax(), SyntaxKind::Record));

    let arguments = parse(rules::ARGUMENT_LIST, "(1,,3)");
    assert_eq!(arguments.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(arguments.consumed.end.0 as usize, "(1,,3)".len());
    assert_eq!(count_kind(&arguments.syntax(), SyntaxKind::CallArgument), 2);
    assert!(contains_kind(&arguments.syntax(), SyntaxKind::Missing));
    assert!(!contains_kind(&arguments.syntax(), SyntaxKind::Error));
}

#[test]
fn transpose_apostrophe_does_not_hide_an_owner_closer() {
    let text = "(1 @ ')";
    let parsed = parse(rules::ARGUMENT_LIST, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Error));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Missing));
}

#[test]
fn speculative_record_selection_restores_the_recovery_budget() {
    let text = "{a: 1, 2: @}";
    let limits = ParseLimits {
        max_recovery_bytes: 1,
        ..ParseLimits::default()
    };
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(text),
        rules::STRUCTURE,
        ParseConfig { limits },
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.stats.recovery_bytes, 1);
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Map));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::Record));
}

#[test]
fn table_separator_recovery_preserves_the_next_row() {
    let text = "|a<u8>|\n|1\n|2|";
    let parsed = parse(rules::TABLE, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::RegularTable));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::InlineTable));
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::TableRow), 2);
}

#[test]
fn whitespace_only_table_row_preserves_the_next_row() {
    let text = "|a<u8>|\n|   \n|2|";
    let parsed = parse(rules::TABLE, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::RegularTable));
    assert_eq!(count_kind(&parsed.syntax(), SyntaxKind::TableRow), 2);
}

#[test]
fn table_selection_defers_inline_recovery_at_a_physical_newline() {
    let text = "|a<u8>|\n|1|";
    let parsed = parse(rules::TABLE, text);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::RegularTable));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::InlineTable));
}

#[test]
fn deferred_inline_table_restores_speculative_recovery_state() {
    let text = "|a<u8> @\n|1|";
    let limits = ParseLimits {
        max_recovery_bytes: 1,
        ..ParseLimits::default()
    };
    let parsed =
        parse_canonical_phase_2i_rule_for_test(source(text), rules::TABLE, ParseConfig { limits })
            .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.consumed.end.0 as usize, text.len());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::RegularTable));
    assert!(!contains_kind(&parsed.syntax(), SyntaxKind::InlineTable));
    assert_eq!(parsed.stats.recovery_bytes, 1);
}

#[test]
fn diagnostics_and_recovery_actions_are_deterministic() {
    let first = parse(rules::ARGUMENT_LIST, "(1 @ [2])");
    let second = parse(rules::ARGUMENT_LIST, "(1 @ [2])");
    assert_eq!(first.diagnostics, second.diagnostics);
    assert_eq!(first.root.structural_hash, second.root.structural_hash);
    assert_eq!(first.consumed, second.consumed);
}

#[test]
fn shared_prefix_recovery_preserves_complete_form_selection() {
    for (text, expected, forbidden) in [
        (
            "(1",
            SyntaxKind::ParentheticalExpression,
            &[SyntaxKind::Tuple][..],
        ),
        (
            "(1,",
            SyntaxKind::Tuple,
            &[SyntaxKind::ParentheticalExpression][..],
        ),
        (
            "[1",
            SyntaxKind::Matrix,
            &[SyntaxKind::MatrixComprehension][..],
        ),
        (
            "[x | x <-",
            SyntaxKind::MatrixComprehension,
            &[SyntaxKind::Matrix][..],
        ),
        (
            "{1",
            SyntaxKind::Set,
            &[
                SyntaxKind::Map,
                SyntaxKind::Record,
                SyntaxKind::SetComprehension,
            ][..],
        ),
        (
            "{1:",
            SyntaxKind::Map,
            &[
                SyntaxKind::Record,
                SyntaxKind::Set,
                SyntaxKind::SetComprehension,
            ][..],
        ),
        (
            "{a:",
            SyntaxKind::Record,
            &[
                SyntaxKind::Map,
                SyntaxKind::Set,
                SyntaxKind::SetComprehension,
            ][..],
        ),
        (
            "{x | x <-",
            SyntaxKind::SetComprehension,
            &[SyntaxKind::Map, SyntaxKind::Record, SyntaxKind::Set][..],
        ),
        (
            "foo(",
            SyntaxKind::FunctionCall,
            &[SyntaxKind::Variable][..],
        ),
        ("x[1", SyntaxKind::Slice, &[SyntaxKind::Variable][..]),
        (
            ":some(1",
            SyntaxKind::TupleStruct,
            &[SyntaxKind::AtomLiteral][..],
        ),
    ] {
        let parsed = parse(rules::EXPRESSION, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
        assert!(
            contains_kind(&parsed.syntax(), expected),
            "{text:?} did not retain {expected:?}"
        );
        for kind in forbidden {
            assert!(
                !contains_kind(&parsed.syntax(), *kind),
                "{text:?} incorrectly retained {kind:?}"
            );
        }
    }
}
