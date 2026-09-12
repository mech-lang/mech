use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, NodeFlags, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode,
    TextSize, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2c5), Revision(0), text).unwrap()
}

fn contains_kind(node: &SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected || node.children().any(|child| contains_kind(&child, expected))
}

fn committed(rule: RuleId, text: &str, parent: SyntaxKind) {
    let parsed =
        parse_canonical_phase_2i_rule_for_test(source(text), rule, ParseConfig::default()).unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
    assert!(!parsed.diagnostics.is_empty(), "{text:?}");
    assert!(contains_kind(&parsed.syntax(), parent), "{text:?}");
    assert!(parsed.root.flags.intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING
    ));
}

#[test]
fn nesting_limit_finishes_balanced_without_stack_overflow() {
    let limits = ParseLimits {
        max_nesting: 8,
        ..ParseLimits::default()
    };
    let text = format!("{}1{}", "(".repeat(64), ")".repeat(64));
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(!parsed.diagnostics.is_empty());
    assert!(parsed.root.flags.intersects(
        NodeFlags::ERROR
            | NodeFlags::MISSING
            | NodeFlags::CONTAINS_ERROR
            | NodeFlags::CONTAINS_MISSING
    ));
}

#[test]
fn fuel_is_a_hard_limit_and_resource_completion_is_balanced() {
    let limits = ParseLimits {
        fuel: 64,
        ..ParseLimits::default()
    };
    let text = core::iter::repeat_n("1", 4_096)
        .collect::<Vec<_>>()
        .join(" + ");
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    assert!(parsed.stats.parser_steps <= limits.fuel);
    assert_eq!(parsed.stats.parser_steps, limits.fuel);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(!parsed.diagnostics.is_empty());
}

#[test]
fn event_budget_is_a_hard_limit_and_resource_completion_is_balanced() {
    let limits = ParseLimits {
        max_events: 32,
        ..ParseLimits::default()
    };
    let text = format!("[{}]", "1 ".repeat(4_096));
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source(&text),
        rules::EXPRESSION,
        ParseConfig { limits },
    )
    .unwrap();
    assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(!parsed.diagnostics.is_empty());
}

#[test]
fn later_committed_children_are_retained_by_every_recursive_repetition() {
    for (rule, text, parent) in [
        (rules::MAP, "{1: 2, 3: \"unterminated", SyntaxKind::Map),
        (
            rules::RECORD,
            "{a: 1, b: \"unterminated",
            SyntaxKind::Record,
        ),
        (rules::SET, "{1, \"unterminated", SyntaxKind::Set),
        (rules::TUPLE, "(1, \"unterminated", SyntaxKind::Tuple),
        (
            rules::ARGUMENT_LIST,
            "(1, \"unterminated",
            SyntaxKind::ArgumentList,
        ),
        (
            rules::INLINE_TABLE_ROW,
            "1 \"unterminated",
            SyntaxKind::InlineTableRow,
        ),
        (
            rules::INLINE_TABLE,
            "|a<u8>|1|2 \"unterminated",
            SyntaxKind::InlineTable,
        ),
        (rules::MATRIX, "[1 \"unterminated", SyntaxKind::Matrix),
        (rules::MATRIX, "[\n1\n\"unterminated", SyntaxKind::Matrix),
    ] {
        committed(rule, text, parent);
    }
}

#[test]
fn low_fuel_in_later_recursive_children_retains_each_parent() {
    let tail = core::iter::repeat_n("1", 512)
        .collect::<Vec<_>>()
        .join(" + ");
    let cases = [
        (rules::MAP, format!("{{1: 2, 3: {tail}}}"), SyntaxKind::Map),
        (
            rules::RECORD,
            format!("{{a: 1, b: {tail}}}"),
            SyntaxKind::Record,
        ),
        (
            rules::INLINE_TABLE_ROW,
            format!("1 {tail}|"),
            SyntaxKind::InlineTableRow,
        ),
        (
            rules::MATRIX,
            format!("[\n1\n{tail}\n]"),
            SyntaxKind::Matrix,
        ),
    ];
    for (rule, text, parent) in cases {
        let limits = ParseLimits {
            fuel: 256,
            ..ParseLimits::default()
        };
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source(&text), rule, ParseConfig { limits })
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{rule:?}");
        assert_eq!(parsed.stats.parser_steps, limits.fuel, "{rule:?}");
        assert!(!parsed.diagnostics.is_empty(), "{rule:?}");
        assert!(contains_kind(&parsed.syntax(), parent), "{rule:?}");
    }
}

#[test]
fn recovery_bytes_are_a_cumulative_hard_limit() {
    let limits = ParseLimits {
        max_recovery_bytes: 1,
        ..ParseLimits::default()
    };
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("(1 + @, 2 + @)"),
        rules::TUPLE,
        ParseConfig { limits },
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.stats.recovery_bytes, 1);
    assert!(!parsed.diagnostics.is_empty());
    assert!(contains_kind(&parsed.syntax(), SyntaxKind::Tuple));
}

#[test]
fn exact_recovery_budget_can_reach_a_restart_boundary() {
    for (unexpected, maximum) in [("@", 1), ("😀", 4)] {
        let text = format!("(1 {unexpected})");
        let limits = ParseLimits {
            max_recovery_bytes: maximum,
            ..ParseLimits::default()
        };
        let parsed = parse_canonical_phase_2i_rule_for_test(
            source(&text),
            rules::PARENTHETICAL_TERM,
            ParseConfig { limits },
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(parsed.consumed.end.0 as usize, text.len());
        assert_eq!(parsed.stats.recovery_bytes, u64::from(maximum));
        assert!(
            parsed
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code.as_str() != "syntax/recovery-limit")
        );
    }
}

#[test]
fn recovery_never_splits_or_overcharges_a_utf8_scalar() {
    let limits = ParseLimits {
        max_recovery_bytes: 3,
        ..ParseLimits::default()
    };
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("(1 😀)"),
        rules::PARENTHETICAL_TERM,
        ParseConfig { limits },
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes));
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit")
    );
}

#[test]
fn diagnostic_limit_applies_to_recursive_recovery() {
    let limits = ParseLimits {
        max_diagnostics: 1,
        ..ParseLimits::default()
    };
    let parsed = parse_canonical_phase_2i_rule_for_test(
        source("(1 +, 2 +)"),
        rules::TUPLE,
        ParseConfig { limits },
    )
    .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert!(parsed.stats.diagnostics_truncated);
}

#[test]
fn adversarial_shared_prefix_recovery_stays_within_fuel_and_byte_limits() {
    let limits = ParseLimits {
        max_recovery_bytes: 128,
        fuel: 1_024,
        ..ParseLimits::default()
    };
    let text = format!("(1 + {})", "@".repeat(8_192));
    let parsed =
        parse_canonical_phase_2i_rule_for_test(source(&text), rules::TUPLE, ParseConfig { limits })
            .unwrap();
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert!(parsed.stats.parser_steps <= limits.fuel);
    assert!(parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes));
    assert!(!parsed.diagnostics.is_empty());
}

#[test]
fn every_recovery_continuation_unwinds_at_each_fuel_and_event_boundary() {
    // Sweep through child starts, tokens, finishes, and the resource envelope;
    // sampling only powers of two missed speculative numeric and owner aborts.
    let cases = [
        (rules::EXPRESSION, "[1 + | x <- xs]"),
        (rules::SET_COMPREHENSION, "{1 + | x <- xs}"),
        (rules::EXPRESSION, "{1: 2, 3:, 4: 5}"),
        (rules::EXPRESSION, "(1,,3)"),
        (rules::EXPRESSION, "{a: 1, (2): 3}"),
        (rules::EXPRESSION, "╭─\n│a<u8│\n│1│"),
        (rules::FSM_PIPE, "# -> :next"),
        (rules::MATCH_ARM, "| (1 +) => 2"),
        (rules::PATTERN_ARRAY, "[1, 2 +, 3]"),
        (rules::INLINE_TABLE, "|a<u8|1|"),
        (rules::FANCY_TABLE, "╭─\n││\n│1│"),
        (rules::KIND_SET, "{<u8}:1:N"),
        (rules::KIND, "{<u8}:1:N"),
        (rules::EXPRESSION, "{a: 1, b⟨u8:1..2⟩}"),
        (rules::EXPRESSION, "{a: 1, b<{u8}:N>}"),
        (rules::L3, "1 + + 3"),
        (rules::EXPRESSION, "[1] + + 3"),
        (rules::EXPRESSION, "{a: 1, b:, c: 3}"),
        (rules::EXPRESSION, "{, 1}"),
        (rules::SUBSCRIPT, "[1 +][2]"),
        (rules::FACTOR, "(1 +)'"),
        (rules::ARGUMENT_LIST, "(1 + @ <u8>, 3)"),
        (rules::MATRIX, "╭1 +╯"),
        (rules::FSM_PIPE, "# ⇒ :value"),
        (rules::FSM_PIPE, "#m -> * → :next"),
        (rules::EXPRESSION, "x ? | => 2 | * => 3"),
    ];
    for (rule, text) in cases {
        let limits = (16..=256)
            .flat_map(|budget| {
                [
                    ParseLimits {
                        fuel: budget,
                        ..ParseLimits::default()
                    },
                    ParseLimits {
                        max_events: budget as u32,
                        ..ParseLimits::default()
                    },
                ]
            })
            .chain((0..=8).map(|max_nesting| ParseLimits {
                max_nesting,
                ..ParseLimits::default()
            }))
            .chain((0..=2).map(|max_diagnostics| ParseLimits {
                max_diagnostics,
                ..ParseLimits::default()
            }))
            .chain(
                (0..=text.len() as u32).map(|max_recovery_bytes| ParseLimits {
                    max_recovery_bytes,
                    ..ParseLimits::default()
                }),
            );
        for limits in limits {
            let parsed = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(source(text), rule, ParseConfig { limits })
                    .unwrap()
            })
            .unwrap_or_else(|_| panic!("{rule:?}, {text:?}, {limits:?}"));
            assert!(parsed.stats.parser_steps <= limits.fuel);
            assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
            assert!(parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes));
            assert!(parsed.diagnostics.len() <= limits.max_diagnostics as usize);
            // Nesting recovery is local and may leave an owner boundary for its
            // caller. Fuel/event exhaustion instead finalizes the full remainder.
            if limits.max_nesting == ParseLimits::default().max_nesting {
                assert_eq!(
                    parsed.consumed,
                    parsed.source.full_range(),
                    "{rule:?} {text:?} {limits:?}"
                );
            } else {
                assert_eq!(parsed.consumed.start, TextSize(0));
                assert!(parsed.source.full_range().contains_range(parsed.consumed));
                if !parsed.matched {
                    // Local recovery in a speculative head can prevent the bar
                    // discriminator from being recognized. A direct comprehension
                    // must then reject transactionally, rather than invent a bar.
                    assert_eq!(rule, rules::SET_COMPREHENSION);
                    assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch);
                    assert_eq!(
                        parsed.consumed,
                        mech_syntax::document::TextRange::empty(TextSize(0))
                    );
                    assert!(parsed.diagnostics.is_empty());
                }
                if parsed
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code.as_str() == "syntax/nesting-limit")
                {
                    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
                }
            }
            mech_syntax::document::validate_lossless_range(
                &parsed.root,
                &parsed.source,
                parsed.consumed,
            )
            .unwrap();
            assert_eq!(
                mech_syntax::document::reconstruct_source_range(
                    &parsed.root,
                    &parsed.source,
                    parsed.consumed
                )
                .unwrap(),
                &text[..parsed.consumed.end.0 as usize]
            );
            let resources = parsed
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit")
                .collect::<Vec<_>>();
            assert!(resources.len() <= 1, "{rule:?} {limits:?}");
            for diagnostic in resources {
                assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
                assert!(parsed.matched);
                assert_eq!(
                    diagnostic.phase,
                    mech_syntax::document::DiagnosticPhase::Syntax
                );
                assert_eq!(diagnostic.severity, mech_syntax::document::Severity::Error);
                assert!(diagnostic.rule.is_some());
                assert_eq!(diagnostic.context, None);
                let Some(mech_syntax::document::RecoveryAction::ResourceLimit { range }) =
                    diagnostic.recovery
                else {
                    panic!("missing resource action");
                };
                assert!(parsed.consumed.contains_range(range));
                assert_eq!(
                    diagnostic
                        .primary
                        .resolve(parsed.source.revision(), &parsed.nodes),
                    Some(range)
                );
            }
        }
    }
}
