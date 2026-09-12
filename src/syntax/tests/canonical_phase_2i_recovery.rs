use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::{canonical_rule_name, rules};
use mech_syntax::document::{
    DocumentId, ExpectedSyntax, NodeFlags, ParseConfig, RecoveryAction, Revision, RuleId,
    SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, reconstruct_source_range,
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
