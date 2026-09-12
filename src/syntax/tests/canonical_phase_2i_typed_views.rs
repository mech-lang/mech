use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    ArgumentListSyntax, ArrayPatternSyntax, AstNode, DocumentId, ExpressionSyntax, FactorSyntax,
    FactorValueSyntax, FormulaSyntax, GreenElement, GreenNode, GreenToken, KindScalarSyntax,
    LiteralSyntax, LiteralValueSyntax, MapSyntax, MatchArmSyntax, MatrixSyntax, NodeFlags, NodeId,
    ParentheticalExpressionSyntax, ParseConfig, ParseLimits, PatternArrayItemSyntax,
    RangeExpressionSyntax, RangeSubscriptSyntax, RecordSyntax, RecursiveCoreSyntax,
    RecursiveSyntaxNode, Revision, SubscriptItemSyntax, SyntaxKind, SyntaxNode, TableKindSyntax,
    TextSize, TextSnapshot, TokenFlags, TokenId, phase_2i_node_kind, text_hash,
};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema_rows() -> Vec<Vec<String>> {
    fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-syntax-schema.tsv"),
    )
    .expect("read Phase 2I syntax schema")
    .lines()
    .skip(1)
    .map(|line| line.split('\t').map(str::to_owned).collect())
    .collect()
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c7), Revision(3), text).unwrap()
}

fn empty_node(kind: SyntaxKind) -> SyntaxNode {
    SyntaxNode::new_root(
        Arc::new(GreenNode {
            id: NodeId(kind as u64 + 1),
            kind,
            text_len: TextSize::ZERO,
            children: Arc::from([]),
            flags: NodeFlags::NONE,
            structural_hash: 0,
        }),
        source(""),
    )
}

fn find_kind(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node.clone());
    }
    node.children().find_map(|child| find_kind(&child, kind))
}

fn find_recovery_wrapped_expression(node: &SyntaxNode) -> Option<ExpressionSyntax> {
    if node.kind() == SyntaxKind::Expression
        && node
            .children()
            .any(|child| child.kind() == SyntaxKind::Expression)
    {
        return ExpressionSyntax::cast(node.clone());
    }
    node.children()
        .find_map(|child| find_recovery_wrapped_expression(&child))
}

fn find_matrix_comprehension_factor(node: &SyntaxNode) -> Option<FactorSyntax> {
    if let Some(factor) = FactorSyntax::cast(node.clone())
        && matches!(
            factor.value(),
            Some(FactorValueSyntax::MatrixComprehension(_))
        )
    {
        return Some(factor);
    }
    node.children()
        .find_map(|child| find_matrix_comprehension_factor(&child))
}

#[test]
fn schema_and_typed_surface_have_one_complete_authority() {
    let rows = schema_rows();
    assert_eq!(rows.len(), 80);
    let mut node_kinds = BTreeSet::new();
    let mut transparent = BTreeSet::new();

    for row in rows {
        assert_eq!(row.len(), 6);
        let name = &row[0];
        match row[2].as_str() {
            "node" | "conditional-node" => {
                let kind = phase_2i_node_kind(name)
                    .unwrap_or_else(|| panic!("missing typed view for {name}"));
                assert_eq!(format!("{kind:?}"), row[3]);
                assert!(node_kinds.insert(kind), "duplicate typed kind {kind:?}");
                let syntax = empty_node(kind);
                let view = RecursiveCoreSyntax::cast(syntax.clone())
                    .unwrap_or_else(|| panic!("typed cast rejected {kind:?}"));
                assert_eq!(view.syntax().kind(), kind);
                assert!(Arc::ptr_eq(view.syntax().green(), syntax.green()));
            }
            "transparent" => {
                assert!(phase_2i_node_kind(name).is_none());
                transparent.insert(name.clone());
            }
            policy => panic!("unknown emission policy {policy}"),
        }
    }

    assert_eq!(node_kinds.len(), 78);
    assert_eq!(
        transparent,
        BTreeSet::from(["formula".to_owned(), "pattern-array-item".to_owned()])
    );
    assert!(!RecursiveCoreSyntax::can_cast(SyntaxKind::Document));
    assert!(RecursiveCoreSyntax::cast(empty_node(SyntaxKind::Document)).is_none());
    assert!(FormulaSyntax::can_cast(SyntaxKind::Factor));
    assert!(!FormulaSyntax::can_cast(SyntaxKind::Expression));
    assert!(PatternArrayItemSyntax::can_cast(SyntaxKind::Pattern));
    assert!(!PatternArrayItemSyntax::can_cast(
        SyntaxKind::ArrayPatternElement
    ));
}

#[test]
fn ordered_children_delimiters_and_trivia_are_available_without_source_copies() {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("(1, x: 2)"),
        rules::ARGUMENT_LIST,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
    let syntax = find_kind(&parsed.syntax(), SyntaxKind::ArgumentList).unwrap();
    let view = ArgumentListSyntax::cast(syntax.clone()).unwrap();

    assert_eq!(view.arguments().len(), 2);
    assert!(view.opening_parenthesis().is_some());
    assert!(view.closing_parenthesis().is_some());
    assert!(!view.trivia_tokens().is_empty());
    assert!(Arc::ptr_eq(view.syntax().green(), syntax.green()));
    assert_eq!(
        view.syntax().source().document(),
        syntax.source().document()
    );
    assert_eq!(
        view.syntax().source().revision(),
        syntax.source().revision()
    );
    assert_eq!(
        view.syntax().source().chunks().next().unwrap().as_ptr(),
        syntax.source().chunks().next().unwrap().as_ptr()
    );

    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("{1:2, 3:4}"),
        rules::MAP,
        ParseConfig::default(),
    )
    .unwrap();
    let map = MapSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Map).unwrap()).unwrap();
    let entries = map.entries();
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .all(|entry| entry.key().is_some() && entry.value().is_some())
    );
    assert!(entries[0].syntax().range().start < entries[1].syntax().range().start);
}

#[test]
fn typed_access_survives_missing_and_error_recovery() {
    let missing = parse_canonical_phase_2i_rule_for_test(
        source("(1"),
        rules::ARGUMENT_LIST,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(missing.outcome, CanonicalRuleOutcome::Committed);
    let view =
        ArgumentListSyntax::cast(find_kind(&missing.syntax(), SyntaxKind::ArgumentList).unwrap())
            .unwrap();
    assert_eq!(view.arguments().len(), 1);
    assert!(view.closing_parenthesis().is_some());
    assert_eq!(view.missing_tokens().len(), 1);

    let unexpected = parse_canonical_phase_2i_rule_for_test(
        source("1 + @"),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(unexpected.outcome, CanonicalRuleOutcome::Committed);
    let expression =
        ExpressionSyntax::cast(find_kind(&unexpected.syntax(), SyntaxKind::Expression).unwrap())
            .unwrap();
    assert!(expression.body().is_some());
    assert_eq!(expression.error_nodes().len(), 1);
    assert!(expression.missing_nodes().is_empty());
}

#[test]
fn typed_roles_follow_parser_boundaries_and_recovery_ownership() {
    let table = parse_canonical_phase_2i_rule_for_test(
        source("|x<u8>|"),
        rules::KIND_TABLE,
        ParseConfig::default(),
    )
    .unwrap();
    let table =
        TableKindSyntax::cast(find_kind(&table.syntax(), SyntaxKind::TableKind).unwrap()).unwrap();
    assert_eq!(table.field_names().len(), 1);
    assert_eq!(table.field_kinds().len(), 1);
    assert!(table.field_kinds()[0].kind().is_some());

    let matrix =
        parse_canonical_phase_2i_rule_for_test(source("[1"), rules::MATRIX, ParseConfig::default())
            .unwrap();
    let matrix =
        MatrixSyntax::cast(find_kind(&matrix.syntax(), SyntaxKind::Matrix).unwrap()).unwrap();
    let closing = matrix
        .closing_delimiter()
        .expect("matrix owns its recovered closing delimiter");
    assert_eq!(closing.kind(), SyntaxKind::RightBracket);
    assert!(closing.flags().contains(TokenFlags::MISSING));
    assert!(matrix.direct_tokens().iter().any(|token| {
        token.kind() == SyntaxKind::RightBracket && token.flags().contains(TokenFlags::MISSING)
    }));

    let arm = parse_canonical_phase_2i_rule_for_test(
        source("| *, x =>"),
        rules::MATCH_ARM,
        ParseConfig::default(),
    )
    .unwrap();
    let arm =
        MatchArmSyntax::cast(find_kind(&arm.syntax(), SyntaxKind::MatchArm).unwrap()).unwrap();
    assert!(arm.guard().is_some());
    assert!(arm.value().is_none());

    let literal = parse_canonical_phase_2i_rule_for_test(
        source("<u8>"),
        rules::LITERAL,
        ParseConfig::default(),
    )
    .unwrap();
    let literal =
        LiteralSyntax::cast(find_kind(&literal.syntax(), SyntaxKind::Literal).unwrap()).unwrap();
    assert!(matches!(
        literal.value(),
        Some(LiteralValueSyntax::KindAnnotation(_))
    ));
    assert!(literal.annotation().is_none());

    let boolean = parse_canonical_phase_2i_rule_for_test(
        source("true<u8>"),
        rules::LITERAL,
        ParseConfig::default(),
    )
    .unwrap();
    let boolean =
        LiteralSyntax::cast(find_kind(&boolean.syntax(), SyntaxKind::Literal).unwrap()).unwrap();
    assert!(boolean.value().is_none());
    assert!(boolean.true_token().is_some());
    assert!(boolean.annotation().is_some());

    let parenthetical =
        parse_canonical_phase_2i_rule_for_test(source("(1"), rules::FACTOR, ParseConfig::default())
            .unwrap();
    let parenthetical = ParentheticalExpressionSyntax::cast(
        find_kind(&parenthetical.syntax(), SyntaxKind::ParentheticalExpression).unwrap(),
    )
    .unwrap();
    assert!(parenthetical.expression().is_some());
    let closing = parenthetical
        .closing_parenthesis()
        .expect("parenthetical owns its recovered closing delimiter");
    assert!(closing.flags().contains(TokenFlags::MISSING));

    let expression = parse_canonical_phase_2i_rule_for_test(
        source("[x | x <- xs]"),
        rules::FACTOR,
        ParseConfig::default(),
    )
    .unwrap();
    let factor = find_matrix_comprehension_factor(&expression.syntax())
        .expect("matrix comprehension factor");
    assert!(matches!(
        factor.value(),
        Some(FactorValueSyntax::MatrixComprehension(_))
    ));

    let recovered = parse_canonical_phase_2i_rule_for_test(
        source("[x | x <- xs"),
        rules::FACTOR,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(recovered.outcome, CanonicalRuleOutcome::Committed);
    assert!(find_kind(&recovered.syntax(), SyntaxKind::Structure).is_none());
    let factor = find_matrix_comprehension_factor(&recovered.syntax())
        .expect("recovered matrix comprehension keeps its selected factor form");
    let Some(FactorValueSyntax::MatrixComprehension(comprehension)) = factor.value() else {
        panic!("recovered factor owns the comprehension directly");
    };
    assert!(comprehension.value().is_some());
    assert_eq!(comprehension.qualifiers().len(), 1);
    assert!(
        comprehension
            .closing_delimiter()
            .unwrap()
            .flags()
            .contains(TokenFlags::MISSING)
    );

    let transposed =
        parse_canonical_phase_2i_rule_for_test(source("x'"), rules::FACTOR, ParseConfig::default())
            .unwrap();
    let transposed =
        FactorSyntax::cast(find_kind(&transposed.syntax(), SyntaxKind::Factor).unwrap()).unwrap();
    assert_eq!(
        transposed.transpose().map(|token| token.kind()),
        Some(SyntaxKind::Apostrophe)
    );
}

#[test]
fn expression_body_descends_through_a_recovery_wrapper() {
    let text = core::iter::repeat_n("1", 512)
        .collect::<Vec<_>>()
        .join(" + ");
    let limits = ParseLimits {
        fuel: 64,
        ..ParseLimits::default()
    };
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    let expression = find_recovery_wrapped_expression(&parsed.syntax())
        .expect("resource recovery retains a nested Expression wrapper");
    assert!(expression.body().is_some());
}

#[test]
fn record_views_expose_physical_and_recovered_delimiters() {
    for (text, opening, closing) in [
        ("{a: 1}", "{", "}"),
        ("|a: 1|", "|", "|"),
        ("╭a: 1╯", "╭", "╯"),
        ("│a: 1│", "│", "│"),
        ("┃a: 1┃", "┃", "┃"),
        ("┌a: 1 ┘", "┌", "┘"),
        ("┏a: 1 ┛", "┏", "┛"),
        ("╭a: 1│", "╭", "│"),
        ("│a: 1|", "│", "|"),
        ("|a: 1┃", "|", "┃"),
    ] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(text),
            rules::RECORD,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(
            parsed.outcome,
            CanonicalRuleOutcome::Matched,
            "{text:?}: {:?}\n{}",
            parsed.diagnostics,
            mech_syntax::document::compact_debug_tree(&parsed.syntax())
        );
        let record =
            RecordSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Record).unwrap()).unwrap();
        assert_eq!(record.opening_delimiter().unwrap().text().unwrap(), opening);
        assert_eq!(record.closing_delimiter().unwrap().text().unwrap(), closing);
        assert_ne!(
            record.opening_delimiter().unwrap().id(),
            record.closing_delimiter().unwrap().id()
        );
    }

    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("{a: 1"),
        rules::RECORD,
        ParseConfig::default(),
    )
    .unwrap();
    let record =
        RecordSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Record).unwrap()).unwrap();
    assert!(
        record
            .closing_delimiter()
            .unwrap()
            .flags()
            .contains(TokenFlags::MISSING)
    );
}

#[test]
fn resource_limited_record_opener_is_not_a_closing_delimiter() {
    for opening in ["{", "|", "│", "┃", "╭", "┌", "┏"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(&format!("{opening}a: 1|")),
            rules::RECORD,
            ParseConfig {
                limits: ParseLimits {
                    max_nesting: 0,
                    ..ParseLimits::default()
                },
            },
        )
        .unwrap();
        let record =
            RecordSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Record).unwrap()).unwrap();
        assert_eq!(record.opening_delimiter().unwrap().text().unwrap(), opening);
        assert!(record.closing_delimiter().is_none());
    }
}

#[test]
fn truncated_vertical_records_expose_synthetic_closers() {
    for opening in ["│", "┃"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(&format!("{opening}a: 1")),
            rules::RECORD,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        let record =
            RecordSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::Record).unwrap()).unwrap();
        let start = record.opening_delimiter().unwrap();
        let end = record.closing_delimiter().unwrap();
        assert_eq!(start.text().unwrap(), opening);
        assert_ne!(start.id(), end.id());
        assert!(!start.flags().contains(TokenFlags::MISSING));
        assert!(end.flags().contains(TokenFlags::MISSING));
        assert!(end.range().is_empty());
    }
}

#[test]
fn range_subscript_exposes_actual_resource_recovery_expression() {
    let text = format!(
        "{}..9",
        core::iter::repeat_n("1", 32)
            .collect::<Vec<_>>()
            .join(" + ")
    );
    for limits in [
        ParseLimits {
            fuel: 64,
            ..ParseLimits::default()
        },
        ParseLimits {
            max_events: 96,
            ..ParseLimits::default()
        },
    ] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(&text),
            rules::RANGE_SUBSCRIPT,
            ParseConfig { limits },
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        let subscript = RangeSubscriptSyntax::cast(
            find_kind(&parsed.syntax(), SyntaxKind::RangeSubscript).unwrap(),
        )
        .unwrap();
        let raw = subscript
            .syntax()
            .children()
            .find(|node| node.kind() == SyntaxKind::Expression)
            .expect("parser retains partial first bound as Expression");
        let recovered = subscript
            .recovered_expression()
            .expect("typed accessor exposes resource-retained expression");
        assert!(subscript.range().is_none());
        assert!(RangeExpressionSyntax::cast(raw.clone()).is_none());
        assert!(recovered.body().is_some());
        assert!(Arc::ptr_eq(raw.green(), recovered.syntax().green()));
        assert_eq!(
            raw.source().chunks().next().unwrap().as_ptr(),
            recovered
                .syntax()
                .source()
                .chunks()
                .next()
                .unwrap()
                .as_ptr()
        );
        assert_eq!(raw.range(), recovered.syntax().range());
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.code.as_str() == "syntax/recovery-limit"
                && matches!(
                    diagnostic.recovery,
                    Some(mech_syntax::document::RecoveryAction::ResourceLimit { .. })
                )
        }));
        assert!(parsed.stats.parser_steps <= limits.fuel);
        assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("1..9"),
        rules::RANGE_SUBSCRIPT,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
    let subscript = RangeSubscriptSyntax::cast(
        find_kind(&parsed.syntax(), SyntaxKind::RangeSubscript).unwrap(),
    )
    .unwrap();
    assert!(subscript.recovered_expression().is_none());
    let range = subscript.range().unwrap();
    assert_eq!(range.bounds().len(), 2);
    assert_eq!(range.operators().len(), 1);
}

#[test]
fn scalar_kind_exposes_actual_resource_recovery_expression() {
    let text = format!(
        "u8:{}..9",
        core::iter::repeat_n("1", 32)
            .collect::<Vec<_>>()
            .join(" + ")
    );
    for limits in [
        ParseLimits {
            fuel: 64,
            ..ParseLimits::default()
        },
        ParseLimits {
            max_events: 96,
            ..ParseLimits::default()
        },
    ] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(&text),
            rules::KIND_SCALAR,
            ParseConfig { limits },
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        let scalar =
            KindScalarSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::KindScalar).unwrap())
                .unwrap();
        let raw = scalar
            .syntax()
            .children()
            .find(|node| node.kind() == SyntaxKind::Expression)
            .expect("parser retains partial first bound as Expression");
        let recovered = scalar
            .recovered_expression()
            .expect("typed accessor exposes resource-retained expression");
        assert!(scalar.constraint().is_none());
        assert!(RangeExpressionSyntax::cast(raw.clone()).is_none());
        assert!(recovered.body().is_some());
        assert!(Arc::ptr_eq(raw.green(), recovered.syntax().green()));
        assert_eq!(
            raw.source().chunks().next().unwrap().as_ptr(),
            recovered
                .syntax()
                .source()
                .chunks()
                .next()
                .unwrap()
                .as_ptr()
        );
        assert_eq!(raw.range(), recovered.syntax().range());
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.code.as_str() == "syntax/recovery-limit"
                && matches!(
                    diagnostic.recovery,
                    Some(mech_syntax::document::RecoveryAction::ResourceLimit { .. })
                )
        }));
        assert!(parsed.stats.parser_steps <= limits.fuel);
        assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("u8:1..9"),
        rules::KIND_SCALAR,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
    let scalar =
        KindScalarSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::KindScalar).unwrap())
            .unwrap();
    assert!(scalar.recovered_expression().is_none());
    let range = scalar.constraint().unwrap();
    assert_eq!(range.bounds().len(), 2);
    assert_eq!(range.operators().len(), 1);
}

#[test]
fn array_pattern_rest_exposes_the_physical_bar() {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("[x | y]"),
        rules::PATTERN_ARRAY,
        ParseConfig::default(),
    )
    .unwrap();
    let array =
        ArrayPatternSyntax::cast(find_kind(&parsed.syntax(), SyntaxKind::ArrayPattern).unwrap())
            .unwrap();
    let rest = array
        .elements()
        .into_iter()
        .find_map(|element| element.rest())
        .expect("array pattern keeps its rest marker");
    assert_eq!(rest.kind(), SyntaxKind::Bar);
    assert_eq!(rest.text().unwrap(), "|");
}

#[test]
fn matrix_closer_rejects_opening_and_decoration_glyphs() {
    let matrix = MatrixSyntax::cast(SyntaxNode::new_root(
        Arc::new(GreenNode {
            id: NodeId(0x44),
            kind: SyntaxKind::Matrix,
            text_len: TextSize(3),
            children: Arc::from([GreenElement::Token(GreenToken {
                id: TokenId(0x45),
                kind: SyntaxKind::BoxDrawing,
                text_len: TextSize(3),
                flags: TokenFlags::NONE,
                text_hash: text_hash("╭"),
            })]),
            flags: NodeFlags::NONE,
            structural_hash: 0,
        }),
        source("╭"),
    ))
    .unwrap();
    assert_eq!(matrix.opening_delimiter().unwrap().text().unwrap(), "╭");
    assert!(matrix.closing_delimiter().is_none());
}

#[test]
fn select_all_is_only_a_nested_subscript_value() {
    assert!(!SubscriptItemSyntax::can_cast(
        SyntaxKind::SelectAllSubscript
    ));
}
