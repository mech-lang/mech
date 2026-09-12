use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, ComprehensionQualifierValueSyntax, DocumentId, MatrixComprehensionSyntax, ParseConfig,
    ParseLimits, RecoveryAction, Revision, SetComprehensionSyntax, SyntaxElement, SyntaxKind,
    SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags, reconstruct_source_range,
    validate_lossless_range,
};
use std::sync::Arc;

fn find(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node.clone());
    }
    node.children().find_map(|node| find(&node, kind))
}

fn snapshot(text: &str, pieces: bool) -> TextSnapshot {
    let source = TextSnapshot::new(DocumentId(821), Revision(8), "").unwrap();
    if pieces {
        text.chars()
            .fold(source, |source, c| source.append(c.to_string()).unwrap())
    } else {
        TextSnapshot::new(DocumentId(821), Revision(8), text).unwrap()
    }
}

fn same_node(actual: &SyntaxNode, expected: &SyntaxNode) {
    assert_eq!(actual.id(), expected.id());
    assert_eq!(actual.range(), expected.range());
    assert!(Arc::ptr_eq(actual.green(), expected.green()));
    assert_eq!(
        actual
            .source()
            .chunks()
            .map(str::as_ptr)
            .collect::<Vec<_>>(),
        expected
            .source()
            .chunks()
            .map(str::as_ptr)
            .collect::<Vec<_>>()
    );
}

#[test]
fn halted_comprehension_exposes_its_retained_head_and_bar() {
    let text = "[x | x <- (1 +)]";
    let parsed = parse_canonical_phase_2i_rule_for_test(
        snapshot(text, false),
        rules::MATRIX_COMPREHENSION,
        ParseConfig {
            limits: ParseLimits {
                fuel: 52,
                ..Default::default()
            },
        },
    )
    .unwrap();
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| matches!(d.recovery, Some(RecoveryAction::ResourceLimit { .. })))
    );
    let node = find(&parsed.syntax(), SyntaxKind::MatrixComprehension).unwrap();
    let physical_head = find(&node, SyntaxKind::Expression).unwrap();
    assert_eq!(physical_head.text().unwrap(), "x");
    let view = MatrixComprehensionSyntax::cast(node.clone()).unwrap();
    same_node(view.value().unwrap().syntax(), &physical_head);
    assert_eq!(
        view.bar().unwrap().range(),
        TextRange::at(TextSize(3), TextSize(1))
    );
    assert_eq!(
        view.opening_delimiter().unwrap().range(),
        TextRange::at(TextSize(0), TextSize(1))
    );
    assert!(view.qualifiers().is_empty());
    // The physical closing byte is retained as error source, not a completed delimiter.
    assert!(view.closing_delimiter().is_none());
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        text
    );
}

#[test]
fn resource_retained_comprehension_roles_keep_order_and_original_storage() {
    let mut wrapped_heads = 0;
    let mut wrapped_qualifiers = 0;
    let mut wrapped_definitions = 0;
    for text in ["[x | x <- (1 +), y := 2]", "[[1 | 2] | x <- [3]]"] {
        let own_bar = text.rfind(" | ").unwrap() as u32 + 1;
        for pieces in [false, true] {
            let source = snapshot(text, pieces);
            for rule in [rules::MATRIX_COMPREHENSION, rules::EXPRESSION] {
                for limits in (0..=400)
                    .map(|fuel| ParseLimits {
                        fuel,
                        ..Default::default()
                    })
                    .chain(
                        (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=240).map(
                            |max_events| ParseLimits {
                                max_events,
                                ..Default::default()
                            },
                        ),
                    )
                {
                    let parsed = parse_canonical_phase_2i_rule_for_test(
                        source.clone(),
                        rule,
                        ParseConfig { limits },
                    )
                    .unwrap();
                    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
                    assert!(parsed.stats.parser_steps <= limits.fuel);
                    assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
                    let Some(node) = find(&parsed.syntax(), SyntaxKind::MatrixComprehension) else {
                        continue;
                    };
                    let view = MatrixComprehensionSyntax::cast(node.clone()).unwrap();
                    let Some(row) = node.children().find(|n| n.kind() == SyntaxKind::MatrixRow)
                    else {
                        continue;
                    };
                    let Some(column) = row
                        .children()
                        .find(|n| n.kind() == SyntaxKind::MatrixColumn)
                    else {
                        continue;
                    };
                    assert!(
                        parsed.diagnostics.iter().any(|d| matches!(
                            d.recovery,
                            Some(RecoveryAction::ResourceLimit { .. })
                        ))
                    );
                    if let Some(head) = column
                        .children()
                        .find(|n| n.kind() == SyntaxKind::Expression)
                    {
                        same_node(view.value().unwrap().syntax(), &head);
                        wrapped_heads += 1;
                    } else {
                        assert!(view.value().is_none());
                    }
                    let physical: Vec<_> = column
                        .children()
                        .filter(|n| n.kind() == SyntaxKind::ComprehensionQualifier)
                        .collect();
                    let typed = view.qualifiers();
                    assert_eq!(typed.len(), physical.len());
                    for (typed, physical) in typed.iter().zip(&physical) {
                        same_node(typed.syntax(), physical);
                        wrapped_qualifiers += 1;
                        if matches!(
                            typed.value(),
                            Some(ComprehensionQualifierValueSyntax::Definition(_))
                        ) {
                            wrapped_definitions += 1;
                        }
                    }
                    let physical_bar =
                        column
                            .children_with_tokens()
                            .into_iter()
                            .find_map(|item| match item {
                                SyntaxElement::Token(token) if token.kind() == SyntaxKind::Bar => {
                                    Some(token)
                                }
                                _ => None,
                            });
                    assert_eq!(view.bar().map(|t| t.id()), physical_bar.map(|t| t.id()));
                    let physical_close = node
                        .children_with_tokens()
                        .into_iter()
                        .chain(column.children_with_tokens())
                        .find_map(|item| match item {
                            SyntaxElement::Token(token)
                                if token.kind() == SyntaxKind::RightBracket =>
                            {
                                Some(token)
                            }
                            _ => None,
                        });
                    if let Some(close) = physical_close {
                        assert_eq!(view.closing_delimiter().unwrap().id(), close.id());
                    }
                    if let Some(bar) = view.bar() {
                        assert_eq!(bar.range(), TextRange::at(TextSize(own_bar), TextSize(1)));
                        assert!(
                            !bar.flags()
                                .intersects(TokenFlags::ERROR | TokenFlags::MISSING)
                        );
                    }
                    if let Some(close) = view.closing_delimiter() {
                        if !close.flags().contains(TokenFlags::MISSING) {
                            assert_eq!(
                                close.range(),
                                TextRange::at(TextSize(text.len() as u32 - 1), TextSize(1))
                            );
                        }
                        assert!(!close.flags().contains(TokenFlags::ERROR));
                    }
                }
            }
        }
    }
    assert!(wrapped_heads > 0);
    assert!(wrapped_qualifiers > 0);
    assert!(wrapped_definitions > 0);
}

#[test]
fn complete_and_recovered_comprehensions_keep_direct_roles() {
    for text in ["[x | x <- [1], y := 2]", "[x | x <- (1 +), y := 2]"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            snapshot(text, true),
            rules::MATRIX_COMPREHENSION,
            ParseConfig::default(),
        )
        .unwrap();
        let node = find(&parsed.syntax(), SyntaxKind::MatrixComprehension).unwrap();
        assert!(node.children().all(|n| n.kind() != SyntaxKind::MatrixRow));
        let view = MatrixComprehensionSyntax::cast(node).unwrap();
        assert_eq!(view.value().unwrap().syntax().text().unwrap(), "x");
        assert_eq!(view.qualifiers().len(), 2);
        assert!(matches!(
            view.qualifiers()[0].value(),
            Some(ComprehensionQualifierValueSyntax::Generator(_))
        ));
        assert!(matches!(
            view.qualifiers()[1].value(),
            Some(ComprehensionQualifierValueSyntax::Definition(_))
        ));
        assert_eq!(
            view.bar().unwrap().range(),
            TextRange::at(TextSize(3), TextSize(1))
        );
        assert_eq!(
            view.closing_delimiter().unwrap().range(),
            TextRange::at(TextSize(text.len() as u32 - 1), TextSize(1))
        );
    }
    let text = "{x | x <- [1], y := 2}";
    let parsed = parse_canonical_phase_2i_rule_for_test(
        snapshot(text, false),
        rules::SET_COMPREHENSION,
        ParseConfig::default(),
    )
    .unwrap();
    let view =
        SetComprehensionSyntax::cast(find(&parsed.syntax(), SyntaxKind::SetComprehension).unwrap())
            .unwrap();
    assert_eq!(view.value().unwrap().syntax().text().unwrap(), "x");
    assert_eq!(view.qualifiers().len(), 2);
    assert_eq!(view.closing_delimiter().unwrap().text().unwrap(), "}");
}
