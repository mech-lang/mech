use mech_syntax::document::parser::canonical::{
    CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, FactorSyntax, FactorValueSyntax, KindMapSyntax, MapSyntax, ParseConfig,
    ParseLimits, Revision, RuleId, StructureSyntax, StructureValueSyntax, SyntaxKind, SyntaxNode,
    TextSnapshot, validate_lossless_range,
};
use std::sync::Arc;

fn nodes(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut out = Vec::new();
    if node.kind() == kind {
        out.push(node.clone());
    }
    for child in node.children() {
        out.extend(nodes(&child, kind));
    }
    out
}
fn direct(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    node.children().find(|child| child.kind() == kind)
}
fn owned_token(node: &SyntaxNode, kind: SyntaxKind) -> Option<mech_syntax::document::SyntaxToken> {
    node.children_with_tokens()
        .into_iter()
        .find_map(|element| match element {
            mech_syntax::document::SyntaxElement::Token(token) if token.kind() == kind => {
                Some(token)
            }
            mech_syntax::document::SyntaxElement::Node(node)
                if node.kind() == SyntaxKind::Missing =>
            {
                node.tokens().into_iter().find(|token| token.kind() == kind)
            }
            _ => None,
        })
}
fn same(actual: &SyntaxNode, expected: &SyntaxNode) {
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
fn sweep(rule: RuleId, text: &str, mut check: impl FnMut(&CanonicalSourceRuleSnapshot)) {
    for pieces in [false, true] {
        let source = if pieces {
            text.chars().fold(
                TextSnapshot::new(DocumentId(821), Revision(10), "").unwrap(),
                |s, c| s.append(c.to_string()).unwrap(),
            )
        } else {
            TextSnapshot::new(DocumentId(821), Revision(10), text).unwrap()
        };
        for limits in (0..=500)
            .map(|fuel| ParseLimits {
                fuel,
                ..Default::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=300).map(
                    |max_events| ParseLimits {
                        max_events,
                        ..Default::default()
                    },
                ),
            )
        {
            let p = parse_canonical_phase_2i_rule_for_test(
                source.clone(),
                rule,
                ParseConfig { limits },
            )
            .unwrap();
            validate_lossless_range(&p.root, &p.source, p.consumed).unwrap();
            assert!(p.stats.parser_steps <= limits.fuel);
            assert!(p.stats.events_emitted <= u64::from(limits.max_events));
            check(&p);
        }
    }
}
#[test]
fn recovered_set_classification_keeps_selected_items_and_delimiters() {
    let mut witnessed = 0;
    for rule in [rules::STRUCTURE, rules::EXPRESSION] {
        for text in ["{1,2 + 3,4}", "{1,{2,3},4}"] {
            sweep(rule, text, |p| {
                for structure in nodes(&p.syntax(), SyntaxKind::Structure) {
                    let Some(map) = direct(&structure, SyntaxKind::Map) else {
                        continue;
                    };
                    let Some(set) = direct(&map, SyntaxKind::Set) else {
                        continue;
                    };
                    let items: Vec<_> = set
                        .children()
                        .filter(|c| c.kind() == SyntaxKind::Expression)
                        .collect();
                    if items.is_empty() || direct(&set, SyntaxKind::SetComprehension).is_some() {
                        continue;
                    }
                    let view = StructureSyntax::cast(structure).unwrap();
                    let Some(StructureValueSyntax::Set(view)) = view.value() else {
                        panic!("selected set lost at fuel {}: {text}", p.stats.parser_steps);
                    };
                    same(view.syntax(), &set);
                    assert_eq!(view.items().len(), items.len());
                    for (a, b) in view.items().iter().zip(&items) {
                        same(a.syntax(), b);
                    }
                    assert_eq!(
                        view.opening_brace().unwrap().range().start,
                        set.range().start
                    );
                    assert_eq!(
                        view.closing_brace().map(|token| token.id()),
                        owned_token(&set, SyntaxKind::RightBrace).map(|token| token.id())
                    );
                    witnessed += 1;
                }
            });
        }
    }
    assert!(witnessed > 0);
}
#[test]
fn recovered_maps_expose_entries_through_retained_owners() {
    let mut witnessed = 0;
    for rule in [rules::EXPRESSION, rules::STRUCTURE] {
        for text in ["{1:2 + 3 * 4}", "{1:{2:3 + 4}}"] {
            sweep(rule, text, |p| {
                for map in nodes(&p.syntax(), SyntaxKind::Map) {
                    let Some(set) = direct(&map, SyntaxKind::Set) else {
                        continue;
                    };
                    let Some(owner) = direct(&set, SyntaxKind::SetComprehension) else {
                        continue;
                    };
                    let physical: Vec<_> = owner
                        .children()
                        .filter(|c| c.kind() == SyntaxKind::MapEntry)
                        .collect();
                    if !physical.iter().any(|entry|entry.children_with_tokens().iter().any(|e|matches!(e,mech_syntax::document::SyntaxElement::Token(t) if t.kind()==SyntaxKind::Colon))) {continue;}
                    let view = MapSyntax::cast(map.clone()).unwrap();
                    assert_eq!(
                        view.entries().len(),
                        physical.len(),
                        "lost map entries at fuel {}: {text}",
                        p.stats.parser_steps
                    );
                    for (a, b) in view.entries().iter().zip(&physical) {
                        same(a.syntax(), b);
                        let expressions: Vec<_> = b
                            .children()
                            .filter(|c| c.kind() == SyntaxKind::Expression)
                            .collect();
                        same(a.key().unwrap().syntax(), &expressions[0]);
                        if let Some(value) = expressions.get(1) {
                            same(a.value().unwrap().syntax(), value);
                        }
                    }
                    assert_eq!(
                        view.opening_brace().unwrap().range().start,
                        map.range().start
                    );
                    assert_eq!(
                        view.closing_brace().map(|token| token.id()),
                        owned_token(&map, SyntaxKind::RightBrace)
                            .or_else(|| owned_token(&owner, SyntaxKind::RightBrace))
                            .map(|token| token.id())
                    );
                    witnessed += 1;
                }
            });
        }
    }
    assert!(witnessed > 0);
}
#[test]
fn recovered_parenthetical_factors_keep_the_inner_expression() {
    let mut witnessed = 0;
    for rule in [rules::FACTOR, rules::EXPRESSION] {
        for text in ["(1 + 2)", "((1 + 2))"] {
            sweep(rule, text, |p| {
                for factor in nodes(&p.syntax(), SyntaxKind::Factor) {
                    let Some(structure) = direct(&factor, SyntaxKind::Structure) else {
                        continue;
                    };
                    let Some(tuple) = direct(&structure, SyntaxKind::Tuple) else {
                        continue;
                    };
                    let Some(parenthetical) = direct(&tuple, SyntaxKind::ParentheticalExpression)
                    else {
                        continue;
                    };
                    if direct(&parenthetical, SyntaxKind::Expression).is_none() {
                        continue;
                    }
                    let Some(FactorValueSyntax::Parenthetical(view)) =
                        FactorSyntax::cast(factor).unwrap().value()
                    else {
                        panic!(
                            "parenthetical hidden at fuel {}: {text}",
                            p.stats.parser_steps
                        );
                    };
                    same(view.syntax(), &parenthetical);
                    assert_eq!(
                        view.opening_parenthesis().unwrap().range().start,
                        parenthetical.range().start
                    );
                    let expression = direct(&parenthetical, SyntaxKind::Expression).unwrap();
                    if let Some(body) = expression.children().find(|child| {
                        <mech_syntax::document::ExpressionBodySyntax as AstNode>::can_cast(
                            child.kind(),
                        )
                    }) {
                        same(view.expression().unwrap().syntax(), &body);
                    }
                    assert_eq!(
                        view.closing_parenthesis().map(|token| token.id()),
                        owned_token(&parenthetical, SyntaxKind::RightParen).map(|token| token.id())
                    );
                    witnessed += 1;
                }
            });
        }
    }
    assert!(witnessed > 0);
}
#[test]
fn recovered_kind_maps_keep_their_key_colon_and_value() {
    let mut witnessed = 0;
    for rule in [rules::KIND, rules::KIND_ANNOTATION] {
        let text = if rule == rules::KIND {
            "{u8:[u8]:2}"
        } else {
            "<{u8:[u8]:2}>"
        };
        sweep(rule, text, |p| {
            for map in nodes(&p.syntax(), SyntaxKind::KindMap) {
                let Some(owner) = direct(&map, SyntaxKind::KindSet) else {
                    continue;
                };
                let Some(colon) = owner
                    .children_with_tokens()
                    .into_iter()
                    .find_map(|e| match e {
                        mech_syntax::document::SyntaxElement::Token(t)
                            if t.kind() == SyntaxKind::Colon =>
                        {
                            Some(t)
                        }
                        _ => None,
                    })
                else {
                    continue;
                };
                let kinds: Vec<_> = owner
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::Kind)
                    .collect();
                if kinds.is_empty() {
                    continue;
                }
                let view = KindMapSyntax::cast(map.clone()).unwrap();
                same(view.key().expect("retained key").syntax(), &kinds[0]);
                assert_eq!(view.colon().unwrap().id(), colon.id());
                if let Some(value) = kinds.get(1) {
                    same(view.value().unwrap().syntax(), value);
                }
                assert_eq!(
                    view.opening_brace().unwrap().range().start,
                    map.range().start
                );
                assert_eq!(
                    view.closing_brace().map(|token| token.id()),
                    owned_token(&owner, SyntaxKind::RightBrace).map(|token| token.id())
                );
                witnessed += 1;
            }
        });
    }
    assert!(witnessed > 0);
}

#[test]
fn retained_owner_access_preserves_clean_and_recovered_roles() {
    for (text, expected) in [
        ("{1,2}", SyntaxKind::Set),
        ("{1:2}", SyntaxKind::Map),
        ("{1:}", SyntaxKind::Map),
        ("(1 + 2)", SyntaxKind::ParentheticalExpression),
        ("(1 +)", SyntaxKind::ParentheticalExpression),
        ("(1,2)", SyntaxKind::Tuple),
    ] {
        let source = TextSnapshot::new(DocumentId(821), Revision(11), text).unwrap();
        let p = parse_canonical_phase_2i_rule_for_test(
            source,
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(p.consumed, p.source.full_range());
        let factor = nodes(&p.syntax(), SyntaxKind::Factor)
            .into_iter()
            .next()
            .unwrap();
        let value = FactorSyntax::cast(factor).unwrap().value().unwrap();
        match value {
            FactorValueSyntax::Structure(structure) => {
                assert_eq!(structure.value().unwrap().syntax().kind(), expected)
            }
            FactorValueSyntax::Parenthetical(parenthetical) => {
                assert_eq!(parenthetical.syntax().kind(), expected)
            }
            other => panic!("unexpected role for {text}: {other:?}"),
        }
    }
}
