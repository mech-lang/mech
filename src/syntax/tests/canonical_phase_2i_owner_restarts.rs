use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
    TokenFlags, reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str, pieces: bool) -> TextSnapshot {
    if pieces {
        text.chars().fold(
            TextSnapshot::new(DocumentId(820), Revision(9), "").unwrap(),
            |s, c| s.append(c.to_string()).unwrap(),
        )
    } else {
        TextSnapshot::new(DocumentId(820), Revision(9), text).unwrap()
    }
}
fn nodes(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut result = Vec::new();
    if node.kind() == kind {
        result.push(node.clone());
    }
    for child in node.children() {
        result.extend(nodes(&child, kind));
    }
    result
}
fn parse(
    rule: RuleId,
    text: &str,
    pieces: bool,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    let p =
        parse_canonical_phase_2i_rule_for_test(source(text, pieces), rule, ParseConfig::default())
            .unwrap();
    validate_lossless_range(&p.root, &p.source, p.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&p.root, &p.source, p.consumed).unwrap(),
        &text[..p.consumed.end.0 as usize]
    );
    p
}
#[test]
fn recovered_set_head_does_not_supply_the_required_bar() {
    for pieces in [false, true] {
        let p = parse(rules::SET_COMPREHENSION, "{(1 +)}", pieces);
        assert_eq!(p.outcome, CanonicalRuleOutcome::NoMatch);
        assert_eq!(p.consumed.end.0, 0);
        assert!(nodes(&p.syntax(), SyntaxKind::SetComprehension).is_empty());
        assert!(p.diagnostics.is_empty());
        let p = parse(rules::EXPRESSION, "{(1 +)}", pieces);
        assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(p.consumed.end.0 as usize, "{(1 +)}".len());
        assert_eq!(nodes(&p.syntax(), SyntaxKind::Set).len(), 1);
        assert!(nodes(&p.syntax(), SyntaxKind::SetComprehension).is_empty());
        let p = parse(rules::SET_COMPREHENSION, "{(1 +) | x <- xs}", pieces);
        assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(p.consumed.end.0 as usize, "{(1 +) | x <- xs}".len());
    }
}
#[test]
fn recovered_first_match_arm_retains_later_arms_and_period() {
    for pieces in [false, true] {
        for text in [
            "x ? @ | * => 1",
            "x ? @ | * => 1.",
            "x ? @ | :a => 1 | * => 2.",
        ] {
            let p = parse(rules::EXPRESSION, text, pieces);
            assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
            assert_eq!(p.consumed.end.0 as usize, text.len(), "{text}");
            assert_eq!(
                nodes(&p.syntax(), SyntaxKind::MatchArm).len(),
                text.matches("=>").count()
            );
            assert_eq!(
                nodes(&p.syntax(), SyntaxKind::IntegerLiteral).len(),
                text.matches("=>").count()
            );
            assert_eq!(
                p.syntax()
                    .tokens()
                    .iter()
                    .filter(|t| t.kind() == SyntaxKind::Period)
                    .count(),
                usize::from(text.ends_with('.'))
            );
        }
    }
}
#[test]
fn missing_middle_range_bound_keeps_the_final_operator_and_bound() {
    for pieces in [false, true] {
        for rule in [
            rules::EXPRESSION,
            rules::RANGE_EXPRESSION,
            rules::RANGE_SUBSCRIPT,
        ] {
            for text in [
                "1..@..3",
                "1..@..=3",
                "1....3",
                "1..@\"..\"..3",
                "1..@(2..4)..3",
            ] {
                let p = parse(rule, text, pieces);
                assert_eq!(
                    p.outcome,
                    CanonicalRuleOutcome::Committed,
                    "{rule:?} {text}"
                );
                assert_eq!(p.consumed.end.0 as usize, text.len(), "{rule:?} {text}");
                assert_eq!(
                    nodes(&p.syntax(), SyntaxKind::RangeOperator).len(),
                    2,
                    "{rule:?} {text}"
                );
                let values: Vec<_> = nodes(&p.syntax(), SyntaxKind::IntegerLiteral)
                    .iter()
                    .map(|n| n.text().unwrap())
                    .collect();
                assert_eq!(values, ["1", "3"], "{rule:?} {text}");
            }
        }
    }
}
#[test]
fn extra_map_separators_retain_later_entries_and_physical_closer() {
    for pieces in [false, true] {
        for rule in [rules::MAP, rules::EXPRESSION, rules::STRUCTURE] {
            for text in ["{1:2,,3:4}", "{1:2,,,3:4}", "{a:2,,3:4}", "{1:2, , 3:4}"] {
                let p = parse(rule, text, pieces);
                assert_eq!(
                    p.outcome,
                    CanonicalRuleOutcome::Committed,
                    "{rule:?} {text}"
                );
                assert_eq!(p.consumed.end.0 as usize, text.len(), "{rule:?} {text}");
                let maps = nodes(&p.syntax(), SyntaxKind::Map);
                assert_eq!(maps.len(), 1, "{rule:?} {text}");
                assert!(
                    nodes(&maps[0], SyntaxKind::MapEntry)
                        .iter()
                        .any(|n| n.text().unwrap() == "3:4"),
                    "{rule:?} {text}"
                );
                let closers: Vec<_> = maps[0]
                    .tokens()
                    .into_iter()
                    .filter(|t| t.kind() == SyntaxKind::RightBrace)
                    .collect();
                assert_eq!(closers.len(), 1);
                assert!(!closers[0].flags().contains(TokenFlags::MISSING));
            }
        }
    }
}
#[test]
fn recovered_map_entries_diagnose_each_extra_separator() {
    for pieces in [false, true] {
        for rule in [rules::MAP, rules::EXPRESSION, rules::STRUCTURE] {
            for (text, missing_entries) in [
                ("{1:(2 +),3:4}", 0),
                ("{1:(2 +),,3:4}", 1),
                ("{1:(2 +),,,3:4}", 2),
                ("{1:(2 +), , 3:4}", 1),
                ("{0:0,1:(2 +),,3:4}", 1),
                ("{a:0,1:(2 +),,3:4}", 1),
                ("{(1 +),3:4}", 0),
                ("{(1 +),,3:4}", 1),
                ("{(1 +),,,3:4}", 2),
                ("{(1 +) , 3:4}", 0),
                ("{0:0,(1 +),3:4}", 0),
                ("{0:0,(1 +),,3:4}", 1),
            ] {
                // Without a colon in the first entry, shared braces select a set.
                if text.starts_with("{(") && rule != rules::MAP {
                    continue;
                }
                let p = parse(rule, text, pieces);
                assert_eq!(p.outcome, CanonicalRuleOutcome::Committed);
                assert_eq!(p.consumed, p.source.full_range(), "{rule:?} {text}");
                assert_eq!(
                    p.diagnostics
                        .iter()
                        .filter(|d| d.code.as_str() == "syntax/missing-map-entry")
                        .count(),
                    missing_entries,
                    "{rule:?} {text} {pieces}: {:?}",
                    p.diagnostics
                );
                assert!(
                    p.diagnostics
                        .iter()
                        .any(|d| d.code.as_str() == "syntax/missing-operator-operand")
                );
                let maps = nodes(&p.syntax(), SyntaxKind::Map);
                assert_eq!(maps.len(), 1);
                let entries = nodes(&maps[0], SyntaxKind::MapEntry);
                assert_eq!(
                    entries
                        .iter()
                        .filter(|entry| entry.text().unwrap().trim() == ","
                            && entry
                                .children()
                                .any(|child| child.kind() == SyntaxKind::Missing))
                        .count(),
                    missing_entries,
                    "{rule:?} {text}"
                );
                assert!(entries.iter().any(|entry| entry.text().unwrap() == "3:4"));
                let closer = maps[0].tokens().into_iter().last().unwrap();
                assert_eq!(closer.kind(), SyntaxKind::RightBrace);
                assert!(!closer.flags().contains(TokenFlags::MISSING));
            }
        }
    }
}
#[test]
fn owner_restarts_preserve_clean_controls() {
    for (rule, text) in [
        (rules::SET_COMPREHENSION, "{x | x <- xs}"),
        (rules::EXPRESSION, "x ? | * => 1."),
        (rules::EXPRESSION, "1..2..=3"),
        (rules::MAP, "{1:2,3:4}"),
    ] {
        let p = parse(rule, text, false);
        assert!(p.is_strictly_clean(), "{text}");
        assert_eq!(p.consumed.end.0 as usize, text.len());
    }
}
#[test]
fn owner_restart_paths_preserve_shared_resource_limits() {
    for (rule, text) in [
        (rules::SET_COMPREHENSION, "{(1 +)}"),
        (rules::EXPRESSION, "x ? @ | * => 1."),
        (rules::EXPRESSION, "1..@..3"),
        (rules::RANGE_SUBSCRIPT, "1..@..3"),
        (rules::MAP, "{1:2,,3:4}"),
        (rules::MAP, "{1:(2 +),,3:4}"),
        (rules::MAP, "{(1 +),,3:4}"),
        (rules::EXPRESSION, "{0:0,(1 +),,3:4}"),
        (rules::EXPRESSION, "{0:0,1:(2 +),,3:4}"),
        (rules::EXPRESSION, "{a:2,,3:4}"),
    ] {
        let limits = (0..=240)
            .map(|fuel| ParseLimits {
                fuel,
                ..Default::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=180).map(
                    |max_events| ParseLimits {
                        max_events,
                        ..Default::default()
                    },
                ),
            )
            .chain((0..=8).flat_map(|max_recovery_bytes| {
                (0..=2).map(move |max_diagnostics| ParseLimits {
                    max_recovery_bytes,
                    max_diagnostics,
                    ..Default::default()
                })
            }));
        for limits in limits {
            let p = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(
                    source(text, true),
                    rule,
                    ParseConfig { limits },
                )
                .unwrap()
            })
            .unwrap_or_else(|_| panic!("{rule:?} {text} {limits:?}"));
            validate_lossless_range(&p.root, &p.source, p.consumed).unwrap();
            assert!(p.stats.parser_steps <= limits.fuel);
            assert!(p.stats.events_emitted <= u64::from(limits.max_events));
            assert!(p.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes));
            assert!(p.stats.diagnostics_emitted <= u64::from(limits.max_diagnostics));
        }
    }
}
