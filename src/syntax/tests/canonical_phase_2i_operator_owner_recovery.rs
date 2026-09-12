//! Recovery retains the recognized operator and comprehension discriminators.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(820), Revision(5), text).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap()
}

fn nodes(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut found = Vec::new();
    if node.kind() == kind {
        found.push(node.clone());
    }
    for child in node.children() {
        found.extend(nodes(&child, kind));
    }
    found
}

fn full(parsed: &CanonicalSourceRuleSnapshot, text: &str) {
    assert_eq!(
        parsed.consumed,
        TextRange::new(TextSize(0), TextSize(text.len() as u32))
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        text
    );
}

// 3997363916
#[test]
fn skipped_invalid_operands_restart_at_the_actual_same_level_operator() {
    for (rule, text, chain_kind, operator_kind, skipped) in [
        (
            rules::L3,
            "1 + @ + 3",
            SyntaxKind::AdditiveExpression,
            SyntaxKind::AddSubOperator,
            "@",
        ),
        (
            rules::EXPRESSION,
            "1 + @ + 3",
            SyntaxKind::AdditiveExpression,
            SyntaxKind::AddSubOperator,
            "@",
        ),
        (
            rules::EXPRESSION,
            "[1] + @ + 3",
            SyntaxKind::AdditiveExpression,
            SyntaxKind::AddSubOperator,
            "@",
        ),
        (
            rules::L4,
            "1 * @ * 3",
            SyntaxKind::MultiplicativeExpression,
            SyntaxKind::MulDivOperator,
            "@",
        ),
        (
            rules::L3,
            "1 + @ \"+\" + 3",
            SyntaxKind::AdditiveExpression,
            SyntaxKind::AddSubOperator,
            "@ \"+\"",
        ),
        (
            rules::L3,
            "1 + @ (2 + 4) + 3",
            SyntaxKind::AdditiveExpression,
            SyntaxKind::AddSubOperator,
            "@ (2 + 4)",
        ),
    ] {
        let p = parse(rule, text);
        full(&p, text);
        assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
        let chains = nodes(&p.syntax(), chain_kind);
        assert_eq!(chains.len(), 1, "{text}");
        let chain = &chains[0];
        assert_eq!(
            chain
                .children()
                .filter(|n| n.kind() == operator_kind)
                .count(),
            2,
            "{text}"
        );
        let factors: Vec<_> = chain
            .children()
            .filter(|n| n.kind() == SyntaxKind::Factor)
            .collect();
        assert_eq!(factors.last().unwrap().text().unwrap(), "3", "{text}");
        let errors = nodes(chain, SyntaxKind::Error);
        assert_eq!(errors.len(), 1, "{text}");
        assert_eq!(errors[0].text().unwrap(), skipped, "{text}");
        assert_eq!(p.diagnostics.len(), 1, "{text}");
        let diagnostic = p.diagnostics.iter().next().unwrap();
        assert_eq!(
            diagnostic.code.as_str(),
            "syntax/unexpected-production-source"
        );
        assert_eq!(
            diagnostic.found.as_ref().unwrap().text.as_deref(),
            Some(skipped)
        );
    }
}

// 3997363915
#[test]
fn recovered_filters_do_not_supply_a_matrix_comprehension_discriminator() {
    for text in ["[x | (1 +)]", "[x | (1 +), true]"] {
        let direct = parse(rules::MATRIX_COMPREHENSION, text);
        assert_eq!(direct.outcome, CanonicalRuleOutcome::NoMatch, "{text}");
        assert_eq!(direct.consumed, TextRange::empty(TextSize(0)));
        assert!(direct.diagnostics.is_empty());
        let shared = parse(rules::EXPRESSION, text);
        full(&shared, text);
        assert!(
            nodes(&shared.syntax(), SyntaxKind::MatrixComprehension).is_empty(),
            "{text}"
        );
        assert_eq!(
            nodes(&shared.syntax(), SyntaxKind::Matrix).len(),
            1,
            "{text}"
        );
    }
}

#[test]
fn recovered_real_generators_and_lets_keep_their_comprehension_owner() {
    for text in [
        "[x | x <- (1 +)]",
        "[x | y := (1 +)]",
        "[x | (1 +), x <- xs]",
        "[x | (1 +), y := 2]",
    ] {
        for rule in [rules::MATRIX_COMPREHENSION, rules::EXPRESSION] {
            let p = parse(rule, text);
            full(&p, text);
            assert_eq!(p.outcome, CanonicalRuleOutcome::Committed, "{text}");
            assert_eq!(
                nodes(&p.syntax(), SyntaxKind::MatrixComprehension).len(),
                1,
                "{text}"
            );
            assert!(nodes(&p.syntax(), SyntaxKind::Matrix).is_empty(), "{text}");
        }
    }
}

#[test]
fn operator_restarts_keep_unicode_and_required_whitespace_grammar() {
    for (rule, text, kind, operator, final_operand) in [
        (
            rules::L4,
            "1 × @ × 3",
            SyntaxKind::MultiplicativeExpression,
            SyntaxKind::MulDivOperator,
            "3",
        ),
        (
            rules::L7,
            "{1} ∪ @ Δ {3}",
            SyntaxKind::SetExpression,
            SyntaxKind::SetOperator,
            "{3}",
        ),
    ] {
        let p = parse(rule, text);
        full(&p, text);
        let chains = nodes(&p.syntax(), kind);
        assert_eq!(chains.len(), 1, "{text}");
        assert_eq!(
            chains[0]
                .children()
                .filter(|n| n.kind() == operator)
                .count(),
            2,
            "{text}"
        );
        assert_eq!(
            chains[0].children().last().unwrap().text().unwrap(),
            final_operand
        );
        assert_eq!(nodes(&chains[0], SyntaxKind::Error)[0].text().unwrap(), "@");
    }
    let text = "{1} ∪ @    \"\"\"x\"\"\" Δ {2}";
    let p = parse(rules::L7, text);
    full(&p, text);
    let chains = nodes(&p.syntax(), SyntaxKind::SetExpression);
    assert_eq!(chains.len(), 1);
    assert_eq!(
        chains[0]
            .children()
            .filter(|n| n.kind() == SyntaxKind::SetOperator)
            .count(),
        2
    );
    assert_eq!(
        nodes(&chains[0], SyntaxKind::Error)[0].text().unwrap(),
        "@    \"\"\"x\"\"\""
    );
    for (rule, text) in [
        (rules::EXPRESSION, "1 + 2 + 3"),
        (rules::EXPRESSION, "[x | true]"),
        (rules::MATRIX_COMPREHENSION, "[x | x <- xs]"),
        (rules::MATRIX_COMPREHENSION, "[x | y := 2]"),
        (rules::SET_COMPREHENSION, "{x | true}"),
    ] {
        let p = parse(rule, text);
        full(&p, text);
        assert!(p.is_strictly_clean(), "{text}: {:?}", p.diagnostics);
    }
}

#[test]
fn invalid_operator_tails_scan_long_trivia_runs_linearly() {
    for trivia in [" \t", "\u{a0}", "\u{2009}"] {
        for later_operator in [false, true] {
            let mut previous = None;
            for length in [64, 128, 256, 512, 1024] {
                let text = format!(
                    "1 + @{}{}",
                    trivia.repeat(length),
                    if later_operator { "+ 3" } else { "#" }
                );
                let p = parse(rules::L3, &text);
                full(&p, &text);
                assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
                assert!(!p.diagnostics.iter().any(|d| matches!(
                    d.recovery,
                    Some(mech_syntax::document::RecoveryAction::ResourceLimit { .. })
                )));
                if let Some((steps, events)) = previous {
                    assert!(
                        p.stats.parser_steps <= steps * 2 + 128,
                        "length={length}: {:?}",
                        p.stats
                    );
                    assert!(
                        p.stats.events_emitted <= events * 2 + 128,
                        "length={length}: {:?}",
                        p.stats
                    );
                }
                previous = Some((p.stats.parser_steps, p.stats.events_emitted));
            }
        }
    }
}

#[test]
fn recovered_operator_and_qualifier_owners_preserve_shared_resource_limits() {
    use mech_syntax::document::ParseLimits;
    for text in [
        "1 + @ + 3",
        "[1] + @ (2 + 4) + 3",
        "{1} ∪ @ Δ {3}",
        "[x | (1 +)]",
        "[x | x <- (1 +)]",
        "[x | y := (1 +)]",
        "[x | y<u8, true]",
    ] {
        let source = text.chars().fold(
            TextSnapshot::new(DocumentId(820), Revision(5), "").unwrap(),
            |snapshot, c| snapshot.append(c.to_string()).unwrap(),
        );
        for limits in (0..=320)
            .map(|fuel| ParseLimits {
                fuel,
                ..Default::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=150).map(
                    |max_events| ParseLimits {
                        max_events,
                        ..Default::default()
                    },
                ),
            )
            .chain((0..=8).map(|max_nesting| ParseLimits {
                max_nesting,
                ..Default::default()
            }))
            .chain((0..=2).flat_map(|max_diagnostics| {
                (0..=16).map(move |max_recovery_bytes| ParseLimits {
                    max_diagnostics,
                    max_recovery_bytes,
                    ..Default::default()
                })
            }))
        {
            let p = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(
                    source.clone(),
                    rules::EXPRESSION,
                    ParseConfig { limits },
                )
            })
            .unwrap_or_else(|_| panic!("{text:?}, {limits:?}: parser panicked"))
            .unwrap_or_else(|| panic!("{text:?}, {limits:?}: missing canonical rule"));
            assert!(p.stats.parser_steps <= limits.fuel, "{text:?}, {limits:?}");
            assert!(
                p.stats.events_emitted <= u64::from(limits.max_events),
                "{text:?}, {limits:?}"
            );
            assert!(
                p.stats.diagnostics_emitted <= u64::from(limits.max_diagnostics),
                "{text:?}, {limits:?}"
            );
            assert!(
                p.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes),
                "{text:?}, {limits:?}"
            );
            validate_lossless_range(&p.root, &p.source, p.consumed)
                .unwrap_or_else(|e| panic!("{text:?}, {limits:?}: {e:?}"));
            assert_eq!(
                reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
                &text[..p.consumed.end.0 as usize]
            );
            for diagnostic in p.diagnostics.iter() {
                assert!(
                    diagnostic
                        .primary
                        .resolve(p.source.revision(), &p.nodes)
                        .is_some(),
                    "{text:?}, {limits:?}: {diagnostic:?}"
                );
            }
        }
    }
}
