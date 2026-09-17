//! Direct and shared-owner evidence for the S2 continuation review.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ExpectedSyntax, ParseConfig, RecoveryAction, Revision, RuleId, SyntaxKind,
    SyntaxNode, TextSize, TextSnapshot, TokenFlags, reconstruct_source_range,
    validate_lossless_range,
};
fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let p = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(820), Revision(3), text).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(
        p.outcome,
        CanonicalRuleOutcome::Committed,
        "{rule:?} {text}: {:?}",
        p.diagnostics
    );
    assert_eq!(
        p.consumed,
        p.source.full_range(),
        "{rule:?} {text}: {:?}",
        p.diagnostics
    );
    validate_lossless_range(&p.root, &p.source, p.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
        text
    );
    for d in p.diagnostics.iter() {
        assert!(d.rule.is_some());
        assert!(d.primary.resolve(p.source.revision(), &p.nodes).is_some());
        assert!(d.recovery.is_some());
    }
    p
}
fn nodes(n: &SyntaxNode, k: SyntaxKind) -> Vec<SyntaxNode> {
    let mut v = Vec::new();
    if n.kind() == k {
        v.push(n.clone());
    }
    for c in n.children() {
        v.extend(nodes(&c, k));
    }
    v
}
fn node(n: &SyntaxNode, k: SyntaxKind) -> SyntaxNode {
    nodes(n, k)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("missing {k:?}"))
}
fn physical(n: &SyntaxNode, text: &str, at: usize) {
    let tokens = n.tokens();
    let t = tokens
        .iter()
        .find(|t| t.text().unwrap() == text && t.range().start.0 as usize == at)
        .unwrap_or_else(|| panic!("missing physical {text} at{at} in {:?}", n.kind()));
    assert!(
        !t.flags()
            .intersects(TokenFlags::ERROR | TokenFlags::MISSING | TokenFlags::SYNTHETIC)
    );
}
fn missing_operand(p: &CanonicalSourceRuleSnapshot, text: &str) {
    let at = text.find('+').unwrap() + 1;
    let at = at + text[at..].len() - text[at..].trim_start().len();
    let diagnostics = p
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "syntax/missing-operator-operand")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    let d = diagnostics[0];
    assert_eq!(d.rule, Some(rules::L3));
    assert_eq!(d.context, None);
    assert_eq!(
        d.expected,
        vec![ExpectedSyntax::Production("expression".into())]
    );
    assert_eq!(
        d.found.as_ref().unwrap().text,
        text[at..].chars().next().map(|c| c.to_string())
    );
    assert_eq!(
        d.recovery,
        Some(RecoveryAction::Insert {
            syntax: ExpectedSyntax::Production("expression".into()),
            at: TextSize(at as u32)
        })
    );
}
fn annotation_nodes_remain_inside_error(node: &SyntaxNode, in_error: bool) {
    let in_error = in_error || node.kind() == SyntaxKind::Error;
    if node.kind() == SyntaxKind::KindAnnotation {
        assert!(in_error, "speculative annotation escaped its ERROR owner");
    }
    for child in node.children() {
        annotation_nodes_remain_inside_error(&child, in_error);
    }
}
#[test]
fn recovered_matrix_columns_keep_later_cells_in_the_same_row() {
    for (rule, text) in [
        (rules::MATRIX_ROW, "1, 2 +, 3"),
        (rules::MATRIX, "[1, 2 +, 3]"),
        (rules::EXPRESSION, "[1, 2 +, 3]"),
        (rules::MATRIX, "╭1, 2 +, 3╯"),
        (rules::MATRIX, "[1, 2 +│ 3]"),
        (rules::MATRIX, "[1, 2 +┃ 3]"),
    ] {
        let p = parse(rule, text);
        missing_operand(&p, text);
        let row = node(&p.syntax(), SyntaxKind::MatrixRow);
        let columns = nodes(&row, SyntaxKind::MatrixColumn);
        assert_eq!(columns.len(), 3, "{text}");
        assert!(columns[2].text().unwrap().contains('3'));
        assert_eq!(nodes(&p.syntax(), SyntaxKind::MatrixRow).len(), 1, "{text}");
        if text.ends_with(']') {
            physical(&node(&p.syntax(), SyntaxKind::Matrix), "]", text.len() - 1);
        }
    }
}
#[test]
fn recovered_mapping_keys_keep_the_colon_value_and_later_entries() {
    for (rule, text) in [
        (rules::MAPPING, "(1 +): 2"),
        (rules::MAP, "{(1 +): 2, 3: 4}"),
        (rules::EXPRESSION, "{(1 +): 2, 3: 4}"),
        (rules::EXPRESSION, "{1: 2, (3 +): 4}"),
        (rules::EXPRESSION, "{1: 2, (3 +): 4, 5: 6}"),
    ] {
        let p = parse(rule, text);
        missing_operand(&p, text);
        let entries = nodes(&p.syntax(), SyntaxKind::MapEntry);
        assert_eq!(
            entries.len(),
            if rule == rules::MAPPING {
                1
            } else if text.contains("5: 6") {
                3
            } else {
                2
            },
            "{text}"
        );
        let recovered = entries
            .iter()
            .find(|e| e.text().unwrap().contains('+'))
            .unwrap();
        physical(recovered, ":", text.find("): ").unwrap() + 1);
        physical(
            recovered,
            if text.contains("(3 +)") { "4" } else { "2" },
            text.find(if text.contains("(3 +)") { ": 4" } else { ": 2" })
                .unwrap()
                + 2,
        );
        if rule != rules::MAPPING {
            assert_eq!(nodes(&p.syntax(), SyntaxKind::Set).len(), 0);
            physical(&node(&p.syntax(), SyntaxKind::Map), "}", text.len() - 1);
        }
    }
}
#[test]
fn recovered_formulas_continue_range_operators_and_later_bounds() {
    for (rule, text, bounds) in [
        (rules::EXPRESSION, "(1 +)..3", 2),
        (rules::RANGE_EXPRESSION, "(1 +)..3", 2),
        (rules::EXPRESSION, "(1 +)..2..3", 3),
        (rules::RANGE_EXPRESSION, "1..(2 +)..3", 3),
        (rules::EXPRESSION, "1..(2 +)..3", 3),
        (rules::RANGE_SUBSCRIPT, "(1 +)..3", 2),
    ] {
        let p = parse(rule, text);
        missing_operand(&p, text);
        let range = node(&p.syntax(), SyntaxKind::RangeExpression);
        assert_eq!(
            nodes(&range, SyntaxKind::RangeOperator).len(),
            bounds - 1,
            "{text}"
        );
        physical(&range, "3", text.len() - 1);
    }
}
#[test]
fn absent_inline_headers_recover_at_the_physical_separator() {
    for sep in ["|", "│", "┃"] {
        let text = format!("{sep}{sep}1{sep}2{sep}");
        for rule in [rules::INLINE_TABLE, rules::EXPRESSION] {
            let p = parse(rule, &text);
            let table = node(&p.syntax(), SyntaxKind::InlineTable);
            assert_eq!(nodes(&table, SyntaxKind::InlineTableRow).len(), 2, "{text}");
            assert_eq!(
                nodes(&p.syntax(), SyntaxKind::InlineTable).len(),
                1,
                "{text}"
            );
            physical(&table, sep, sep.len());
            let d = p
                .diagnostics
                .iter()
                .find(|d| d.code.as_str() == "syntax/missing-inline-table-header")
                .unwrap();
            assert_eq!(d.rule, Some(rules::INLINE_TABLE));
            assert_eq!(d.context, None);
            assert_eq!(
                d.expected,
                vec![ExpectedSyntax::Production("inline-table-header".into())]
            );
            assert_eq!(d.found.as_ref().unwrap().text.as_deref(), Some(sep));
            assert_eq!(
                d.recovery,
                Some(RecoveryAction::Insert {
                    syntax: ExpectedSyntax::Production("inline-table-header".into()),
                    at: TextSize(sep.len() as u32)
                })
            );
            assert_eq!(p.diagnostics.len(), 1);
            assert_eq!(nodes(&table, SyntaxKind::Missing).len(), 1);
            assert!(
                table
                    .children()
                    .any(|child| child.kind() == SyntaxKind::Missing)
            );
            assert!(nodes(&table, SyntaxKind::Error).is_empty());
        }
    }
}
#[test]
fn nested_ordinary_delimiters_preserve_framed_ancestor_closers() {
    for (open, close) in [("╭", "╯"), ("┌", "┘"), ("┏", "┛")] {
        for interior in ["(1 + @ ", "f(1 + @ "] {
            let text = format!("{open}{interior}{close}");
            for rule in [rules::MATRIX, rules::EXPRESSION] {
                let p = parse(rule, &text);
                let matrix = node(&p.syntax(), SyntaxKind::Matrix);
                physical(&matrix, close, text.len() - close.len());
                assert!(!p.diagnostics.iter().any(|d| {
                    d.fixes
                        .iter()
                        .any(|fix| fix.edits.iter().any(|e| e.insert.contains(close)))
                }));
            }
        }
    }
}
#[test]
fn bare_less_than_comparisons_do_not_hide_later_siblings() {
    for middle in ["a < b", "a<b", "a< b", "a <b"] {
        let text = format!("(1 + @ {middle}, 3)");
        for rule in [rules::ARGUMENT_LIST, rules::TUPLE, rules::EXPRESSION] {
            let p = parse(rule, &text);
            let error = node(&p.syntax(), SyntaxKind::Error);
            assert_eq!(
                error.text().unwrap(),
                &text[text.find('@').unwrap()..text.find(',').unwrap()]
            );
            assert_eq!(p.diagnostics.len(), 1);
            let diagnostic = p.diagnostics.iter().next().unwrap();
            assert_eq!(
                diagnostic.code.as_str(),
                "syntax/unexpected-production-source"
            );
            assert_eq!(diagnostic.rule, Some(rules::L3));
            assert_eq!(diagnostic.context, None);
            assert!(diagnostic.expected.is_empty());
            assert_eq!(
                diagnostic.found.as_ref().unwrap().kind,
                Some(SyntaxKind::Unknown)
            );
            assert_eq!(
                diagnostic.found.as_ref().unwrap().text.as_deref(),
                Some(error.text().unwrap().as_str())
            );
            assert_eq!(
                diagnostic.recovery,
                Some(RecoveryAction::Abandon {
                    rule: rules::L3,
                    at: error.range().end
                })
            );
            let parent = if rule == rules::ARGUMENT_LIST {
                SyntaxKind::ArgumentList
            } else {
                SyntaxKind::Tuple
            };
            let owner = node(&p.syntax(), parent);
            physical(&owner, "3", text.len() - 2);
            physical(&owner, ")", text.len() - 1);
            assert_eq!(
                nodes(
                    &owner,
                    if parent == SyntaxKind::ArgumentList {
                        SyntaxKind::CallArgument
                    } else {
                        SyntaxKind::Expression
                    }
                )
                .iter()
                .filter(|n| n.text().unwrap() == "3")
                .count(),
                1,
                "{text}"
            );
        }
    }
}
#[test]
fn continuation_recovery_preserves_clean_language_and_real_angle_annotations() {
    for (rule, text) in [
        (rules::EXPRESSION, "[1,2,3]"),
        (rules::EXPRESSION, "1..2..3"),
        (rules::EXPRESSION, "{1:2,3:4}"),
        (rules::EXPRESSION, "|a<bool>|true || false|"),
        (rules::EXPRESSION, "[x | x <- xs]"),
        (rules::EXPRESSION, "{x | x <- xs}"),
    ] {
        let p = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(820), Revision(3), text).unwrap(),
            rule,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(p.is_strictly_clean(), "{text}: {:?}", p.diagnostics);
        assert_eq!(p.consumed, p.source.full_range());
    }
    for annotation in [
        "<[u8]:1,2>",
        "⟨[u8]:1,2⟩",
        "⟨[u8]:1,2>",
        "<[u8]:1,2⟩",
        "<<[u8]:1,2>>",
    ] {
        let text = format!("(1 + @ {annotation}, 3)");
        let p = parse(rules::ARGUMENT_LIST, &text);
        let args = nodes(&p.syntax(), SyntaxKind::CallArgument);
        assert_eq!(args.len(), 2, "{text}");
        physical(&args[1], "3", text.len() - 2);
        let error = node(&args[0], SyntaxKind::Error);
        assert!(error.text().unwrap().contains(annotation), "{text}");
    }
}
#[test]
fn all_six_structure_continuations_keep_shared_limits_and_source_ownership() {
    use mech_syntax::document::ParseLimits;
    for (rule, text) in [
        (rules::EXPRESSION, "[1, 2 +, 3]"),
        (rules::EXPRESSION, "{(1 +): 2, 3: 4}"),
        (rules::EXPRESSION, "(1 +)..2..3"),
        (rules::EXPRESSION, "||1|2|"),
        (rules::EXPRESSION, "╭(1 + @ ╯"),
        (rules::ARGUMENT_LIST, "(1 + @ a<b, 3)"),
        (rules::ARGUMENT_LIST, "(1 + @ <<[u8]:1,2>>, 3)"),
        (rules::ARGUMENT_LIST, "(1 + @ <<@>>, 3)"),
        (rules::ARGUMENT_LIST, "(1 + @ <[<@]:1,2>, 3)"),
    ] {
        for limits in (0..=300)
            .map(|fuel| ParseLimits {
                fuel,
                ..ParseLimits::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=180).map(
                    |max_events| ParseLimits {
                        max_events,
                        ..ParseLimits::default()
                    },
                ),
            )
            .chain((0..=12).map(|max_nesting| ParseLimits {
                max_nesting,
                ..ParseLimits::default()
            }))
            .chain((0..=3).flat_map(|max_diagnostics| {
                (0..=32).map(move |max_recovery_bytes| ParseLimits {
                    max_diagnostics,
                    max_recovery_bytes,
                    ..ParseLimits::default()
                })
            }))
        {
            let p = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(
                    TextSnapshot::new(DocumentId(820), Revision(3), text).unwrap(),
                    rule,
                    ParseConfig { limits },
                )
                .unwrap()
            })
            .unwrap_or_else(|_| panic!("{rule:?}, {text:?}, {limits:?}"));
            assert!(p.stats.parser_steps <= limits.fuel, "{text}: {limits:?}");
            assert!(
                p.stats.events_emitted <= u64::from(limits.max_events),
                "{text}: {limits:?}"
            );
            assert!(
                p.stats.diagnostics_emitted <= u64::from(limits.max_diagnostics),
                "{text}: {limits:?}"
            );
            assert!(
                p.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes),
                "{text}: {limits:?}"
            );
            if text.contains("@ <") {
                annotation_nodes_remain_inside_error(&p.syntax(), false);
            }
            validate_lossless_range(&p.root, &p.source, p.consumed)
                .unwrap_or_else(|e| panic!("{text}, {limits:?}: {e:?}"));
            assert_eq!(
                reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
                &text[..p.consumed.end.0 as usize],
                "{text}: {limits:?}"
            );
        }
    }
}
