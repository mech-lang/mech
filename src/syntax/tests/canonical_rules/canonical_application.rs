//! Ordinary application syntax has an explicit typed owner and exact counts.
use crate::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_document_rule_for_test,
    parse_canonical_executable_rule_for_test,
};
use crate::document::parser::rules;
use crate::document::{
    ApplicationCountError, AstNode, DocumentId, ExpressionBodySyntax, ExpressionSyntax,
    ParentheticalExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x6a11), Revision(1), text).unwrap()
}
fn find(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node.clone());
    }
    node.children().find_map(|child| find(&child, kind))
}
fn expression(text: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_executable_rule_for_test(
        source(text),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{text}");
    assert!(
        parsed.is_strictly_clean(),
        "{text}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.consumed, parsed.source.full_range());
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    ExpressionSyntax::cast(find(&parsed.syntax(), SyntaxKind::Expression).unwrap()).unwrap()
}
#[test]
fn application_has_full_seed_and_ordered_exact_stages() {
    let value = expression("[x + 1u64 | x <- [1u64 2u64]] |> first |19> next |> last");
    let ExpressionBodySyntax::Application(body) = value.body().unwrap() else {
        panic!()
    };
    assert!(matches!(
        body.seed(),
        Some(ExpressionBodySyntax::MatrixComprehension(_))
    ));
    let stages = body.stages();
    assert_eq!(stages.len(), 3);
    for (stage, name, count) in [
        (&stages[0], "first", None),
        (&stages[1], "next", Some(19)),
        (&stages[2], "last", None),
    ] {
        assert_eq!(stage.function().unwrap().syntax().text().unwrap(), name);
        assert_eq!(stage.count(), Ok(count));
    }
}
#[test]
fn count_is_checked_decimal_u64_and_zero_is_retained() {
    for (text, count) in [
        ("seed |0> step", 0),
        ("seed |25> step", 25),
        ("seed |18446744073709551615> step", u64::MAX),
    ] {
        let value = expression(text);
        let ExpressionBodySyntax::Application(body) = value.body().unwrap() else {
            panic!()
        };
        assert_eq!(body.stages()[0].count(), Ok(Some(count)));
    }
    let value = expression("seed |18446744073709551616> step");
    let ExpressionBodySyntax::Application(body) = value.body().unwrap() else {
        panic!()
    };
    assert_eq!(
        body.stages()[0].count(),
        Err(ApplicationCountError::Overflow)
    );
}

#[test]
fn bracketed_counts_reuse_matrix_expressions_and_keep_stage_order() {
    for count in [
        "[2]",
        "[n]",
        "[n * 2]",
        "[5 4 3 2 1]",
        "[1 1 1 1 1]",
        "[1 3 2 5 3]",
        "[1; 3]",
    ] {
        let text = format!("seed |{count}> step |> next");
        let value = expression(&text);
        let ExpressionBodySyntax::Application(body) = value.body().unwrap() else {
            panic!("application");
        };
        assert_eq!(body.stages().len(), 2);
        let count_expression = body.stages()[0].count_expression().unwrap();
        assert_eq!(count_expression.syntax().text().unwrap(), count);
        assert_eq!(body.stages()[1].count(), Ok(None));
    }
}
#[test]
fn grouped_complete_expression_retains_match_and_application_owners() {
    let value = expression("(flag ? | true => 1u64 | false => 2u64) |> next");
    let ExpressionBodySyntax::Application(body) = value.body().unwrap() else {
        panic!()
    };
    let seed = body.seed().unwrap();
    let grouped = ParentheticalExpressionSyntax::cast(
        find(seed.syntax(), SyntaxKind::ParentheticalExpression).unwrap(),
    )
    .unwrap();
    assert_eq!(grouped.complete_expression().unwrap().match_arms().len(), 2);
    let value = expression("(seed |> next)");
    let grouped = ParentheticalExpressionSyntax::cast(
        find(value.syntax(), SyntaxKind::ParentheticalExpression).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        grouped.complete_expression().unwrap().body(),
        Some(ExpressionBodySyntax::Application(_))
    ));
}
#[test]
fn expression_result_definition_is_distinct_from_named_outputs_and_pattern_arms() {
    for (text, expected) in [
        (
            "step(x<[f64]>) => <[f64]> := x |> apply |> apply-transpose",
            SyntaxKind::FunctionDefineExpression,
        ),
        (
            "step(x<u64>) => (out<u64>) := out := x + 1u64.",
            SyntaxKind::FunctionDefineStatements,
        ),
        (
            "step(x<u64>) => <u64> | value => value + 1u64.",
            SyntaxKind::FunctionDefineMatchArms,
        ),
    ] {
        let parsed = parse_canonical_document_rule_for_test(
            source(text),
            rules::FUNCTION_DEFINE,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(
            parsed.outcome,
            CanonicalRuleOutcome::Matched,
            "{text}: {:?}",
            parsed.diagnostics
        );
        assert!(
            parsed.diagnostics.is_empty(),
            "{text}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.consumed, parsed.source.full_range());
        assert!(find(&parsed.syntax(), expected).is_some());
    }
}
#[test]
fn committed_application_recovery_retains_operator_and_never_consumes_other_bars() {
    for text in ["seed |>", "seed |19", "seed |19>"] {
        let parsed = parse_canonical_executable_rule_for_test(
            source(text),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text}");
        assert!(!parsed.diagnostics.is_empty());
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    }
    assert!(matches!(
        expression("[x | x <- [1u64]]").body(),
        Some(ExpressionBodySyntax::MatrixComprehension(_))
    ));
    assert_eq!(
        expression("flag ? | true => 1u64 | false => 2u64")
            .match_arms()
            .len(),
        2
    );
}

#[test]
fn numeric_table_rows_and_comprehension_comparisons_are_not_counted_stages() {
    for text in [
        "(|a<u8> l<u8>|1u8 7u8|2u8 8u8|)",
        "(|a<f64> b<bool>|signal<f64> true|9 false|)",
        "table/join((|a<u8> l<u8>|1u8 7u8|2u8 8u8|), (|a<u8> r<u8>|1u8 9u8|3u8 10u8|))",
        "[x | x <- [1u64 2u64], x > 0u64]",
    ] {
        let value = expression(text);
        assert!(
            find(value.syntax(), SyntaxKind::ApplicationExpression).is_none(),
            "{text}"
        );
    }
}

#[test]
fn integer_kind_suffixes_do_not_select_imaginary_unit_prefixes() {
    use crate::document::parse_canonical_document;
    for number in [
        "7i8",
        "7i16",
        "7i32",
        "7i64",
        "7i128",
        "9223372036854775807i64",
        "18446744073709551615u64",
        "7integer",
        "7j64",
        "7i🦀",
    ] {
        let parsed = parse_canonical_executable_rule_for_test(
            source(number),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(
            parsed.is_strictly_clean(),
            "{number}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.consumed, parsed.source.full_range(), "{number}");
        assert!(
            find(&parsed.syntax(), SyntaxKind::TypedInteger).is_some(),
            "{number}"
        );
        assert!(
            find(&parsed.syntax(), SyntaxKind::ComplexNumber).is_none(),
            "{number}"
        );
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
        let text = format!("value := {number}\nvalue\nlive<i64>\n");
        let document = parse_canonical_document(source(&text), ParseConfig::default());
        assert!(
            document.is_strictly_clean(),
            "{text}: {:?}",
            document.diagnostics
        );
        assert_eq!(
            find(&document.syntax(), SyntaxKind::VariableDefine)
                .unwrap()
                .text()
                .unwrap(),
            format!("value := {number}")
        );
        validate_lossless_range(
            &document.root,
            &document.source,
            document.source.full_range(),
        )
        .unwrap();
    }
    for number in ["7i", "7j", "2+3i", "2-3j", "2i+3j", "2i * 3", "2i/3"] {
        let parsed = parse_canonical_executable_rule_for_test(
            source(number),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(
            parsed.is_strictly_clean(),
            "{number}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.consumed, parsed.source.full_range(), "{number}");
        assert!(
            find(&parsed.syntax(), SyntaxKind::ComplexNumber).is_some(),
            "{number}"
        );
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    }
}
