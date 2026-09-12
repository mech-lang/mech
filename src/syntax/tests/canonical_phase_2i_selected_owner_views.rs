use mech_syntax::document::parser::canonical::{
    CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, FactorSyntax, FactorValueSyntax, ParseConfig, ParseLimits, Revision,
    RuleId, StructureSyntax, StructureValueSyntax, SyntaxKind, SyntaxNode, TextSnapshot,
    validate_lossless_range,
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
fn halted_tuple_does_not_select_a_provisional_parenthetical() {
    let mut witnessed = 0;
    for rule in [rules::FACTOR, rules::EXPRESSION] {
        for text in ["(1,2)", "(1,)", "([1,2],3)"] {
            sweep(rule, text, |p| {
                // This boundary halts inside the first item, before the parser
                // can tentatively select a completed formula as parenthetical.
                if p.stats.parser_steps != 1 {
                    return;
                }
                for factor in nodes(&p.syntax(), SyntaxKind::Factor) {
                    if factor.range().start.0 != 0 {
                        continue;
                    }
                    let Some(structure) = direct(&factor, SyntaxKind::Structure) else {
                        continue;
                    };
                    let Some(tuple) = direct(&structure, SyntaxKind::Tuple) else {
                        continue;
                    };
                    if direct(&tuple, SyntaxKind::ParentheticalExpression).is_none() {
                        continue;
                    }
                    let Some(FactorValueSyntax::Structure(view)) =
                        FactorSyntax::cast(factor).unwrap().value()
                    else {
                        panic!(
                            "tuple misclassified at step {}: {text}",
                            p.stats.parser_steps
                        );
                    };
                    same(view.syntax(), &structure);
                    let Some(StructureValueSyntax::Tuple(value)) = view.value() else {
                        panic!("tuple role lost");
                    };
                    same(value.syntax(), &tuple);
                    witnessed += 1;
                }
            });
        }
    }
    assert!(witnessed > 0);
}
#[test]
fn missing_only_sets_keep_their_selected_role() {
    let mut witnessed = 0;
    for rule in [rules::EXPRESSION, rules::STRUCTURE] {
        for text in ["{,}", "{,,}", "{, }", "{, @}"] {
            sweep(rule, text, |p| {
                for structure in nodes(&p.syntax(), SyntaxKind::Structure) {
                    let Some(map) = direct(&structure, SyntaxKind::Map) else {
                        continue;
                    };
                    let Some(set) = direct(&map, SyntaxKind::Set) else {
                        continue;
                    };
                    if direct(&set, SyntaxKind::Missing).is_none()
                        || direct(&set, SyntaxKind::Expression).is_some()
                    {
                        continue;
                    }
                    let Some(StructureValueSyntax::Set(view)) =
                        StructureSyntax::cast(structure).unwrap().value()
                    else {
                        panic!(
                            "missing-only set misclassified at step {}: {text}",
                            p.stats.parser_steps
                        );
                    };
                    same(view.syntax(), &set);
                    assert!(view.items().is_empty());
                    assert_eq!(
                        view.opening_brace().map(|t| t.id()),
                        owned_token(&set, SyntaxKind::LeftBrace).map(|t| t.id())
                    );
                    assert_eq!(
                        view.closing_brace().map(|t| t.id()),
                        owned_token(&set, SyntaxKind::RightBrace).map(|t| t.id())
                    );
                    witnessed += 1;
                }
            });
        }
    }
    assert!(witnessed > 0);
}
#[test]
fn kind_selection_exposes_sets_and_records_beneath_provisional_maps() {
    let mut sets = 0;
    let mut records = 0;
    for text in ["{u8}", "{[u8]:2}", "{a<u8>}", "{a<u8> b<u16>}"] {
        for rule in [rules::KIND, rules::KIND_ANNOTATION] {
            let source = if rule == rules::KIND_ANNOTATION {
                format!("<{text}>")
            } else {
                text.to_owned()
            };
            sweep(rule, &source, |p| {
                for kind in nodes(&p.syntax(), SyntaxKind::Kind) {
                    let Some(map) = direct(&kind, SyntaxKind::KindMap) else {
                        continue;
                    };
                    let Some(set) = direct(&map, SyntaxKind::KindSet) else {
                        continue;
                    };
                    if owned_token(&set, SyntaxKind::Colon).is_some() {
                        continue;
                    }
                    let value = mech_syntax::document::KindSyntax::cast(kind)
                        .unwrap()
                        .value();
                    if let Some(record) = direct(&set, SyntaxKind::KindRecord) {
                        if direct(&record, SyntaxKind::Identifier).is_none() {
                            continue;
                        }
                        let Some(mech_syntax::document::KindValueSyntax::Record(view)) = value
                        else {
                            panic!("kind record hidden: {text}");
                        };
                        same(view.syntax(), &record);
                        records += 1;
                    } else {
                        if direct(&set, SyntaxKind::Kind).is_none() {
                            continue;
                        }
                        let Some(mech_syntax::document::KindValueSyntax::Set(view)) = value else {
                            panic!("kind set hidden: {text}");
                        };
                        same(view.syntax(), &set);
                        sets += 1;
                    }
                }
            });
        }
    }
    assert!(sets > 0 && records > 0);
}

#[test]
fn provisional_selection_flags_are_local_and_absent_from_clean_trees() {
    use mech_syntax::document::NodeFlags;
    let source = TextSnapshot::new(DocumentId(821), Revision(12), "(1,2)").unwrap();
    let p = parse_canonical_phase_2i_rule_for_test(
        source,
        rules::FACTOR,
        ParseConfig {
            limits: ParseLimits {
                fuel: 1,
                ..Default::default()
            },
        },
    )
    .unwrap();
    let factor = nodes(&p.syntax(), SyntaxKind::Factor)
        .into_iter()
        .next()
        .unwrap();
    let structure = direct(&factor, SyntaxKind::Structure).unwrap();
    let tuple = direct(&structure, SyntaxKind::Tuple).unwrap();
    let parenthetical = direct(&tuple, SyntaxKind::ParentheticalExpression).unwrap();
    assert!(parenthetical.flags().contains(NodeFlags::PROVISIONAL));
    for selected in [&factor, &structure, &tuple] {
        assert!(!selected.flags().contains(NodeFlags::PROVISIONAL));
    }
    fn clean(node: &SyntaxNode) {
        assert!(!node.flags().contains(NodeFlags::PROVISIONAL));
        for child in node.children() {
            clean(&child);
        }
    }
    for (rule, text) in [
        (rules::EXPRESSION, "(1,2)"),
        (rules::EXPRESSION, "(1)"),
        (rules::EXPRESSION, "{1:2}"),
        (rules::EXPRESSION, "{1,2}"),
        (rules::KIND, "{u8:u16}"),
    ] {
        let source = TextSnapshot::new(DocumentId(821), Revision(12), text).unwrap();
        let p =
            parse_canonical_phase_2i_rule_for_test(source, rule, ParseConfig::default()).unwrap();
        assert!(p.is_strictly_clean(), "{text}");
        clean(&p.syntax());
    }
}
