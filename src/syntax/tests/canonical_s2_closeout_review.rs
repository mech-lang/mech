//! S2 closeout regressions for recursion limits and seeded recovery continuation.
//! The four original review cases failed on reviewed head 0058540c3.

use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
    reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str, limits: ParseLimits) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(820), Revision(99), text)
        .expect("small review fixture fits the source range");
    parse_canonical_phase_2i_rule_for_test(source, rule, ParseConfig { limits })
        .expect("review fixture selects an existing S2 rule")
}

fn assert_lossless_and_bounded(parsed: &CanonicalSourceRuleSnapshot, limits: ParseLimits) {
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
        .expect("recovery must retain a valid lossless prefix");
    let actual = reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed)
        .expect("reconstruct the parsed prefix");
    let expected = parsed
        .source
        .text(parsed.consumed)
        .expect("valid source range");
    assert_eq!(actual, expected);
    assert!(parsed.stats.parser_steps <= limits.fuel);
    assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
    assert!(parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes));
    assert!(parsed.diagnostics.len() <= limits.max_diagnostics as usize);
}

fn assert_depth_recovery(parsed: &CanonicalSourceRuleSnapshot, text: &str, limits: ParseLimits) {
    assert_lossless_and_bounded(parsed, limits);
    let text: String = text.chars().take(100).collect();
    assert_eq!(
        parsed.outcome,
        CanonicalRuleOutcome::Committed,
        "recursive input exceeded max_nesting but did not recover: {text:?}"
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code.as_str() == "syntax/nesting-limit" }),
        "recursive input bypassed the configured nesting guard: {text:?}; {:?}",
        parsed.diagnostics
    );
    // Local nesting recovery can legitimately leave an enclosing boundary.
    // Do not require full consumption here, unlike the continuation tests below.
}

fn find(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node.clone());
    }
    node.children().find_map(|child| find(&child, kind))
}

fn assert_outer_addition_retained(rule: RuleId, text: &str, final_operand: &str) {
    let limits = ParseLimits::default();
    let parsed = parse(rule, text, limits);
    assert_lossless_and_bounded(&parsed, limits);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed, "{text:?}");
    assert_eq!(
        parsed.consumed,
        parsed.source.full_range(),
        "recovery left the valid outer addition outside its expression: {text:?}"
    );
    assert!(
        !parsed.diagnostics.is_empty(),
        "original recovery must remain"
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.code.as_str() != "syntax/recovery-limit" }),
        "ordinary continuation must not be hidden by resource finalization"
    );
    let expected_code = if text.contains('@') {
        "syntax/unexpected-production-source"
    } else if text.starts_with("{a:") {
        "syntax/missing-binding-value"
    } else {
        "syntax/missing-operator-operand"
    };
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == expected_code),
        "original recovery diagnostic changed: {text}"
    );
    let expression = parsed.syntax();
    let additive = find(&expression, SyntaxKind::AdditiveExpression)
        .expect("the physical outer plus must own an additive expression");
    assert_eq!(additive.range().end, parsed.source.full_range().end);
    assert!(
        additive.children().any(|child| {
            child.kind() == SyntaxKind::AddSubOperator
                && child.text().is_ok_and(|text| text.trim() == "+")
        }),
        "a synthetic wrapper or ERROR tail is not operator continuation: {text:?}"
    );
    assert_eq!(
        additive
            .children()
            .last()
            .expect("outer right operand")
            .text()
            .unwrap()
            .trim(),
        final_operand,
        "the final operand must remain a child of the outer addition: {text:?}"
    );
}

#[test]
fn unary_recursion_obeys_the_configured_nesting_limit() {
    let limits = ParseLimits {
        max_nesting: 4,
        ..ParseLimits::default()
    };
    for (rule, prefix, leaf) in [
        (rules::EXPRESSION, "-", "1"),
        (rules::NEGATE_FACTOR, "-", "1"),
        (rules::EXPRESSION, "!", "true"),
        (rules::NOT_FACTOR, "!", "true"),
    ] {
        let text = format!("{}{leaf}", prefix.repeat(32));
        let parsed = parse(rule, &text, limits);
        assert_depth_recovery(&parsed, &text, limits);
    }
}

#[test]
fn nested_match_results_obey_the_configured_nesting_limit() {
    let limits = ParseLimits {
        max_nesting: 4,
        ..ParseLimits::default()
    };
    let text = format!("{}1", "x ? | * => ".repeat(16));
    let parsed = parse(rules::EXPRESSION, &text, limits);
    assert_depth_recovery(&parsed, &text, limits);
}

#[test]
fn recovered_leading_collections_continue_into_outer_operators() {
    for text in ["[1 +] + 2", "{a:} + 2"] {
        // FORMULA exercises ordinary precedence; EXPRESSION exercises the seed.
        // The record example is a syntax recovery test, not a type-valid program.
        for rule in [rules::FORMULA, rules::EXPRESSION] {
            assert_outer_addition_retained(rule, text, "2");
        }
    }
}

#[test]
fn recovered_seeded_inner_precedence_continues_into_outer_precedence() {
    for text in [
        "1 * * 2 + 3",
        "1 * @ * 2 + 3",
        "[1] * * 2 + 3",
        "[1] * @ * 2 + 3",
    ] {
        for rule in [rules::FORMULA, rules::EXPRESSION] {
            assert_outer_addition_retained(rule, text, "3");
        }
    }
}

#[test]
fn mixed_fsm_and_table_recursion_share_the_nesting_limit() {
    let limits = ParseLimits {
        max_nesting: 4,
        ..Default::default()
    };
    let cases = [
        format!("{}1{}", "-(".repeat(16), ")".repeat(16)),
        format!("{}:done", "#m -> ".repeat(16)),
        format!("{}1{}", "|a<*>|".repeat(16), "|".repeat(16)),
    ];
    for text in cases {
        let p = parse(rules::EXPRESSION, &text, limits);
        assert_depth_recovery(&p, &text, limits);
    }
    // Sibling depth is restored: a long flat list is not a recursive chain.
    let text = format!("({})", vec!["-1"; 32].join(","));
    let p = parse(rules::EXPRESSION, &text, limits);
    assert!(p.is_strictly_clean());
    assert_eq!(p.consumed, p.source.full_range());
}

#[test]
fn recursive_input_stress_runs_in_a_separate_process() {
    const CHILD: &str = "MECH_S2_CLOSEOUT_STRESS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let limits = ParseLimits {
            max_nesting: 8,
            ..Default::default()
        };
        for text in [
            format!("{}1", "-".repeat(4096)),
            format!("{}true", "!".repeat(4096)),
            format!("{}1", "x ? | * => ".repeat(4096)),
            format!("{}:done", "#m -> ".repeat(4096)),
            format!("{}1{}", "|a<*>|".repeat(4096), "|".repeat(4096)),
            format!("{}1{}", "-(".repeat(4096), ")".repeat(4096)),
        ] {
            let p = parse(rules::EXPRESSION, &text, limits);
            assert_depth_recovery(&p, &text, limits);
        }
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "recursive_input_stress_runs_in_a_separate_process",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stress child failed: {:?}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn closeout_continuations_and_depth_guards_keep_all_resource_limits() {
    for text in [
        "-----1",
        "x ? | * => x ? | * => 1",
        "[1 +] + 2",
        "{a:} + 2",
        "[1] * @ * 2 + 3",
        "[1 +]..2..3",
        "[1 +] ? | * => 2",
    ] {
        for rule in [rules::EXPRESSION, rules::FORMULA] {
            let budgets = (0..=180)
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
                .chain((0..=8).map(|max_nesting| ParseLimits {
                    max_nesting,
                    ..Default::default()
                }))
                .chain((0..=8).flat_map(|max_recovery_bytes| {
                    (0..=2).map(move |max_diagnostics| ParseLimits {
                        max_recovery_bytes,
                        max_diagnostics,
                        ..Default::default()
                    })
                }));
            for limits in budgets {
                let p = std::panic::catch_unwind(|| parse(rule, text, limits))
                    .unwrap_or_else(|_| panic!("{rule:?} {text} {limits:?}"));
                assert_lossless_and_bounded(&p, limits);
            }
        }
    }
}
