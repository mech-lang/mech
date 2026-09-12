//! Later-child and candidate-selection regressions from the S2 review.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DiagnosticPhase, DocumentId, ExpectedSyntax, ParseConfig, RecoveryAction, Revision, RuleId,
    Severity, SyntaxElement, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
    reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let p = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(820), Revision(4), text).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap();
    validate_lossless_range(&p.root, &p.source, p.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
        &text[..p.consumed.end.0 as usize]
    );
    p
}

fn nodes(n: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut found = Vec::new();
    if n.kind() == kind {
        found.push(n.clone());
    }
    for child in n.children() {
        found.extend(nodes(&child, kind));
    }
    found
}

fn only(n: &SyntaxNode, kind: SyntaxKind) -> SyntaxNode {
    let found = nodes(n, kind);
    assert_eq!(found.len(), 1, "{kind:?}: {n:?}");
    found[0].clone()
}
fn direct(n: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    n.children().filter(|n| n.kind() == kind).collect()
}
fn full(p: &CanonicalSourceRuleSnapshot) {
    assert_eq!(p.consumed, p.source.full_range(), "{:?}", p.diagnostics);
    for d in p.diagnostics.iter() {
        assert!(d.rule.is_some());
        assert!(d.primary.resolve(p.source.revision(), &p.nodes).is_some());
    }
}
fn physical(n: &SyntaxNode, text: &str, at: usize) {
    let t = n
        .children_with_tokens()
        .into_iter()
        .find_map(|e| match e {
            SyntaxElement::Token(t)
                if t.range().start.0 as usize == at && t.text().unwrap() == text =>
            {
                Some(t)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing direct physical {text:?} at {at} in {:?}", n.kind()));
    assert!(
        !t.flags()
            .intersects(TokenFlags::ERROR | TokenFlags::MISSING | TokenFlags::SYNTHETIC)
    );
}
fn inserted(
    p: &CanonicalSourceRuleSnapshot,
    code: &str,
    rule: RuleId,
    at: usize,
    expected: ExpectedSyntax,
    found: &str,
) {
    assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(p.diagnostics.len(), 1, "{:?}", p.diagnostics);
    assert_eq!(nodes(&p.syntax(), SyntaxKind::Error).len(), 0);
    let missing = only(&p.syntax(), SyntaxKind::Missing);
    let range = TextRange::empty(TextSize(at as u32));
    assert_eq!(missing.range(), range);
    let d = p.diagnostics.iter().next().unwrap();
    assert_eq!(d.code.as_str(), code);
    assert_eq!(d.phase, DiagnosticPhase::Syntax);
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.rule, Some(rule));
    assert_eq!(d.context, None);
    assert_eq!(
        d.primary.resolve(p.source.revision(), &p.nodes),
        Some(range)
    );
    assert_eq!(d.expected, vec![expected.clone()]);
    assert_eq!(d.found.as_ref().unwrap().text.as_deref(), Some(found));
    assert_eq!(
        d.recovery,
        Some(RecoveryAction::Insert {
            syntax: expected,
            at: TextSize(at as u32)
        })
    );
}

// 3997054268
#[test]
fn framed_header_continues_after_recovered_field() {
    for sep in ["│", "┃", "|"] {
        for header in [
            format!("a<u8{sep}b<u8>{sep}c{sep}"),
            format!("a{sep}b<u8{sep}c<u8>{sep}"),
        ] {
            for rule in [
                rules::FANCY_TABLE_HEADER,
                rules::FANCY_TABLE,
                rules::TABLE,
                rules::EXPRESSION,
            ] {
                let text = if rule == rules::FANCY_TABLE_HEADER {
                    header.clone()
                } else {
                    format!("╭─\n{sep}{header}\n{sep}1{sep}2{sep}3{sep}")
                };
                let p = parse(rule, &text);
                full(&p);
                let owner = only(&p.syntax(), SyntaxKind::FancyTableHeader);
                let fields = direct(&owner, SyntaxKind::TableField);
                assert_eq!(fields.len(), 3, "{text}");
                assert_eq!(
                    fields[2].text().unwrap(),
                    if header.starts_with("a<u8") {
                        "c"
                    } else {
                        "c<u8>"
                    }
                );
                let at = text.find("<u8").unwrap() + 3;
                inserted(
                    &p,
                    "syntax/missing-delimiter",
                    rules::KIND_ANNOTATION,
                    at,
                    ExpectedSyntax::Token(SyntaxKind::RightAngle),
                    sep,
                );
                let recovered = fields
                    .iter()
                    .find(|f| !nodes(f, SyntaxKind::Missing).is_empty())
                    .unwrap();
                let annotation = only(recovered, SyntaxKind::KindAnnotation);
                let missing = only(&annotation, SyntaxKind::Missing);
                let tokens = missing.tokens();
                assert_eq!(tokens.len(), 1);
                assert_eq!(tokens[0].kind(), SyntaxKind::RightAngle);
                assert!(
                    tokens[0]
                        .flags()
                        .contains(TokenFlags::MISSING | TokenFlags::SYNTHETIC)
                );
                for (at, _) in text.match_indices(sep).filter(|(at, _)| {
                    *at >= owner.range().start.0 as usize && *at < owner.range().end.0 as usize
                }) {
                    physical(&owner, sep, at);
                }
                if rule != rules::FANCY_TABLE_HEADER {
                    let row = only(&p.syntax(), SyntaxKind::FancyTableRow);
                    assert_eq!(direct(&row, SyntaxKind::Expression).len(), 3);
                }
            }
        }
    }
}

// 3997054270
#[test]
fn framed_row_continues_after_recovered_cell() {
    for sep in ["│", "┃", "|"] {
        for row in [
            format!("{sep}1{sep}2 +{sep}3{sep}"),
            format!("{sep}1 +{sep}2{sep}3{sep}"),
        ] {
            for rule in [rules::TABLE_ROW2, rules::FANCY_TABLE, rules::EXPRESSION] {
                let text = if rule == rules::TABLE_ROW2 {
                    row.clone()
                } else {
                    format!("╭─\n{sep}a{sep}b{sep}c{sep}\n{row}")
                };
                let p = parse(rule, &text);
                full(&p);
                let owner = only(&p.syntax(), SyntaxKind::FancyTableRow);
                let cells = direct(&owner, SyntaxKind::Expression);
                assert_eq!(cells.len(), 3, "{text}");
                assert_eq!(cells[2].text().unwrap(), "3");
                inserted(
                    &p,
                    "syntax/missing-operator-operand",
                    rules::L3,
                    text.find('+').unwrap() + 1,
                    ExpectedSyntax::Production("expression".into()),
                    sep,
                );
                assert!(
                    nodes(
                        &cells[if row.starts_with(&format!("{sep}1 +")) {
                            0
                        } else {
                            1
                        }],
                        SyntaxKind::Missing
                    )
                    .len()
                        == 1
                );
                for (at, _) in text
                    .match_indices(sep)
                    .filter(|(at, _)| *at >= owner.range().start.0 as usize)
                {
                    physical(&owner, sep, at);
                }
            }
        }
    }
}

// 3997054273
#[test]
fn required_range_does_not_select_a_recovered_formula() {
    for text in ["(1 +)", "1 +"] {
        for rule in [rules::RANGE_EXPRESSION, rules::RANGE_SUBSCRIPT] {
            let p = parse(rule, text);
            assert_eq!(p.outcome, CanonicalRuleOutcome::NoMatch);
            assert_eq!(p.consumed, TextRange::empty(TextSize(0)));
            assert!(p.diagnostics.is_empty());
            assert!(nodes(&p.syntax(), SyntaxKind::Missing).is_empty());
            assert!(nodes(&p.syntax(), SyntaxKind::RangeSubscript).is_empty());
        }
    }
    for (rule, text, count) in [
        (rules::BRACKET_SUBSCRIPT, "[1 +]", 1),
        (rules::SUBSCRIPT, "[1 +][2]", 2),
        (rules::EXPRESSION, "x[1 +][2]", 2),
    ] {
        let p = parse(rule, text);
        full(&p);
        assert_eq!(nodes(&p.syntax(), SyntaxKind::RangeSubscript).len(), 0);
        assert_eq!(nodes(&p.syntax(), SyntaxKind::RangeExpression).len(), 0);
        assert_eq!(
            nodes(&p.syntax(), SyntaxKind::FormulaSubscript).len(),
            count
        );
        inserted(
            &p,
            "syntax/missing-operator-operand",
            rules::L3,
            text.find('+').unwrap() + 1,
            ExpectedSyntax::Production("expression".into()),
            "]",
        );
        let brackets = nodes(&p.syntax(), SyntaxKind::BracketSubscript);
        for bracket in brackets {
            physical(&bracket, "]", bracket.range().end.0 as usize - 1);
        }
    }
    for text in ["(1 +)..3", "1..(2 +)..3"] {
        let p = parse(rules::RANGE_EXPRESSION, text);
        full(&p);
        assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(nodes(&p.syntax(), SyntaxKind::RangeExpression).len(), 1);
    }
}

// 3997054274
#[test]
fn kind_map_selects_colon_after_recovered_key() {
    for rule in [rules::KIND_MAP, rules::KIND, rules::KIND_ANNOTATION] {
        let text = if rule == rules::KIND_ANNOTATION {
            "<{[<u8]:u16}>"
        } else {
            "{[<u8]:u16}"
        };
        let p = parse(rule, text);
        full(&p);
        let map = only(&p.syntax(), SyntaxKind::KindMap);
        assert_eq!(nodes(&p.syntax(), SyntaxKind::KindSet).len(), 0);
        let kinds = direct(&map, SyntaxKind::Kind);
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds[0].text().unwrap(), "[<u8]");
        assert_eq!(kinds[1].text().unwrap(), "u16");
        physical(&map, ":", text.find(':').unwrap());
        physical(&map, "}", text.find('}').unwrap());
        let key = only(&kinds[0], SyntaxKind::KindMatrix);
        physical(&key, "]", text.find(']').unwrap());
        inserted(
            &p,
            "syntax/missing-delimiter",
            rules::KIND_KIND,
            text.find(']').unwrap(),
            ExpectedSyntax::Token(SyntaxKind::RightAngle),
            "]",
        );
    }
}

// 3997054276
#[test]
fn tight_comparison_key_selects_shared_map() {
    for key in [
        "(b<c)",
        "(b<=c)",
        "(b>c)",
        "(b<c+d)",
        "(b< c)",
        "(b<u8>)",
        "(b⟨u8⟩)",
        "(b⟨u8>)",
        "\"a:<b,c>\"",
    ] {
        for rule in [rules::MAP, rules::STRUCTURE, rules::EXPRESSION] {
            let text = format!("{{a: 1, {key}: 2, d: 3}}");
            let p = parse(rule, &text);
            full(&p);
            assert!(p.is_strictly_clean(), "{text}: {:?}", p.diagnostics);
            let map = only(&p.syntax(), SyntaxKind::Map);
            assert!(nodes(&p.syntax(), SyntaxKind::Record).is_empty());
            let entries = direct(&map, SyntaxKind::MapEntry);
            assert_eq!(entries.len(), 3);
            let key_expr = direct(&entries[1], SyntaxKind::Expression)[0].clone();
            assert_eq!(key_expr.text().unwrap(), key);
            physical(&entries[1], ":", text.find(": 2").unwrap());
            physical(&map, "}", text.len() - 1);
        }
    }
}

// 3997054278
#[test]
fn operand_recovery_preserves_match_suffix() {
    for text in [
        "1 + ? | * => 2",
        "[1] + ? | * => 2",
        "{1} + ? | * => 2",
        "(1 +) ? | * => 2",
    ] {
        let p = parse(rules::EXPRESSION, text);
        full(&p);
        let arm = only(&p.syntax(), SyntaxKind::MatchArm);
        assert_eq!(arm.text().unwrap(), "| * => 2");
        let expression = direct(&p.syntax(), SyntaxKind::Expression)[0].clone();
        physical(&expression, "?", text.find('?').unwrap());
        let at = if text.contains("+)") {
            text.find(')').unwrap()
        } else {
            text.find('?').unwrap()
        };
        inserted(
            &p,
            "syntax/missing-operator-operand",
            if text.starts_with(['[', '{']) {
                rules::EXPRESSION
            } else {
                rules::L3
            },
            at,
            ExpectedSyntax::Production("expression".into()),
            if text.contains("+)") { ")" } else { "?" },
        );
    }
    let text = "1 + @ ? | * => 2";
    let p = parse(rules::EXPRESSION, text);
    full(&p);
    let expression = direct(&p.syntax(), SyntaxKind::Expression)[0].clone();
    physical(&expression, "?", 6);
    assert_eq!(
        only(&expression, SyntaxKind::MatchArm).text().unwrap(),
        "| * => 2"
    );
    assert_eq!(only(&expression, SyntaxKind::Error).text().unwrap(), "@ ");
    assert!(nodes(&expression, SyntaxKind::Missing).is_empty());
    assert_eq!(p.diagnostics.len(), 1);
    let d = p.diagnostics.iter().next().unwrap();
    assert_eq!(d.code.as_str(), "syntax/unexpected-production-source");
    assert_eq!(d.rule, Some(rules::L3));
    assert_eq!(d.context, None);
    assert!(d.expected.is_empty());
    assert_eq!(d.found.as_ref().unwrap().text.as_deref(), Some("@ "));
    assert_eq!(
        d.recovery,
        Some(RecoveryAction::Abandon {
            rule: rules::L3,
            at: TextSize(6)
        })
    );
}

#[test]
fn later_continuations_preserve_clean_controls_and_direct_annotation_recovery() {
    let mut previous = None;
    for size in [32, 64, 128, 256] {
        let text = vec!["a"; size].join("<");
        let p = parse(rules::EXPRESSION, &text);
        assert!(p.is_strictly_clean(), "{text}: {:?}", p.diagnostics);
        full(&p);
        if let Some((steps, events)) = previous {
            assert!(p.stats.parser_steps <= steps * 2 + 512);
            assert!(p.stats.events_emitted <= events * 2 + 512);
        }
        previous = Some((p.stats.parser_steps, p.stats.events_emitted));
    }
    for (rule, text) in [
        (rules::FANCY_TABLE_HEADER, "a<u8>│b<u8>│"),
        (rules::TABLE_ROW2, "│1│2│3│"),
        (rules::KIND_MATRIX, "[u8]:"),
        (rules::KIND_MATRIX, "[u8]:1,2"),
        (rules::KIND_MAP, "{[u8]:1:u16}"),
        (rules::KIND, "{[u8]:1:u16}"),
        (rules::KIND, "{u8}:1:N"),
        (rules::VAR, "b<u8>"),
        (rules::EXPRESSION, "b<c"),
        (rules::EXPRESSION, "b<c>"),
        (rules::EXPRESSION, "1 ? | * => 2"),
        (rules::EXPRESSION, "[1] ? | * => 2"),
        (rules::EXPRESSION, "(1)..3"),
        (rules::EXPRESSION, "x[1..3]"),
    ] {
        let p = parse(rule, text);
        full(&p);
        assert!(p.is_strictly_clean(), "{text}: {:?}", p.diagnostics);
    }
    let p = parse(rules::VAR, "b<c");
    full(&p);
    assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
    let annotation = only(&p.syntax(), SyntaxKind::KindAnnotation);
    assert_eq!(
        only(&annotation, SyntaxKind::Missing).range(),
        TextRange::empty(TextSize(3))
    );
    assert_eq!(
        p.diagnostics.iter().next().unwrap().rule,
        Some(rules::KIND_ANNOTATION)
    );
}

#[test]
fn nested_annotation_map_keys_have_linear_probe_growth() {
    let mut previous = None;
    for size in [8, 16, 32, 64] {
        let text = format!("{{a:1,(b{}u8{}):2}}", "<".repeat(size), ">".repeat(size));
        let p = parse(rules::EXPRESSION, &text);
        full(&p);
        assert!(p.is_strictly_clean(), "{text}: {:?}", p.diagnostics);
        let map = only(&p.syntax(), SyntaxKind::Map);
        assert_eq!(direct(&map, SyntaxKind::MapEntry).len(), 2);
        if let Some((steps, events)) = previous {
            assert!(p.stats.parser_steps <= steps * 2 + 512);
            assert!(p.stats.events_emitted <= events * 2 + 512);
        }
        previous = Some((p.stats.parser_steps, p.stats.events_emitted));
    }
}

#[test]
fn all_later_continuations_retain_bounded_owned_source_under_shared_limits() {
    use mech_syntax::document::ParseLimits;
    for (rule, text) in [
        (rules::FANCY_TABLE_HEADER, "a<u8│b<u8>│c│"),
        (rules::EXPRESSION, "╭─\n│a<u8│b<u8>│\n│1│2│"),
        (rules::TABLE_ROW2, "│1│2 +│3│"),
        (rules::RANGE_EXPRESSION, "(1 +)"),
        (rules::SUBSCRIPT, "[1 +][2]"),
        (rules::KIND_MAP, "{[<u8]:u16}"),
        (rules::KIND, "{[<u8]:u16}"),
        (rules::EXPRESSION, "{a:1,(b<c):2}"),
        (rules::EXPRESSION, "{a:1,(b<<[u8]:1,2>>):2}"),
        (rules::EXPRESSION, "{a:1,(b<<@>>):2}"),
        (rules::EXPRESSION, "1 + @ ? | * => 2"),
        (rules::EXPRESSION, "[1] + ? | * => 2"),
    ] {
        for limits in (0..=500)
            .map(|fuel| ParseLimits {
                fuel,
                ..ParseLimits::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=240).map(
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
                (0..=24).map(move |max_recovery_bytes| ParseLimits {
                    max_diagnostics,
                    max_recovery_bytes,
                    ..ParseLimits::default()
                })
            }))
        {
            let p = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(
                    TextSnapshot::new(DocumentId(820), Revision(4), text).unwrap(),
                    rule,
                    ParseConfig { limits },
                )
                .unwrap()
            })
            .unwrap_or_else(|_| panic!("{rule:?} {text:?} {limits:?}"));
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
            validate_lossless_range(&p.root, &p.source, p.consumed)
                .unwrap_or_else(|e| panic!("{text}: {limits:?}: {e:?}"));
            assert_eq!(
                reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
                &text[..p.consumed.end.0 as usize],
                "{text}: {limits:?}"
            );
            for d in p.diagnostics.iter() {
                assert!(d.rule.is_some(), "{text}: {limits:?}: {d:?}");
                assert!(
                    d.primary.resolve(p.source.revision(), &p.nodes).is_some(),
                    "{text}: {limits:?}: {d:?}"
                );
            }
        }
    }
}

#[test]
fn halted_annotation_candidate_keeps_its_enclosing_owner() {
    let text = "{a:1,(b<c):2}";
    let p = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(820), Revision(4), text).unwrap(),
        rules::EXPRESSION,
        ParseConfig {
            limits: mech_syntax::document::ParseLimits {
                fuel: 25,
                ..Default::default()
            },
        },
    )
    .unwrap();
    full(&p);
    assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
    let annotation = only(&p.syntax(), SyntaxKind::KindAnnotation);
    assert!(!nodes(&annotation, SyntaxKind::Error).is_empty());
    validate_lossless_range(&p.root, &p.source, p.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
        text
    );
}
