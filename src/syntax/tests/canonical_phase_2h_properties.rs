use mech_syntax::document::parser::canonical::{
    CanonicalSourceRuleSnapshot, parse_canonical_phase_2h_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, TextRange, TextSize,
    TextSnapshot, TokenFlags, compact_debug_tree, normalize_diagnostics, reconstruct_source_range,
    validate_lossless_range,
};
use proptest::prelude::*;

const PHASE_2H_RULES: &[RuleId] = &[
    rules::MATRIX_START,
    rules::MATRIX_END,
    rules::TABLE_START,
    rules::TABLE_END,
    rules::TABLE_SEPARATOR,
    rules::TABLE_HORZ,
    rules::TABLE_TOP,
    rules::ROW_SEPARATOR,
    rules::EMPTY_MAP,
    rules::EMPTY_SET,
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2b), Revision(0), text).unwrap()
}

fn piece_source(parts: &[&str]) -> TextSnapshot {
    let mut source = source("");
    for part in parts {
        source = source.append((*part).to_owned()).unwrap();
    }
    assert_eq!(source.piece_count(), parts.len());
    source
}

fn parse(source: TextSnapshot, rule: RuleId, config: ParseConfig) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2h_rule_for_test(source, rule, config)
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2H direct rule"))
}

fn token_signature(
    parsed: &CanonicalSourceRuleSnapshot,
) -> Vec<(SyntaxKind, TextRange, TokenFlags, String)> {
    parsed
        .syntax()
        .tokens()
        .into_iter()
        .map(|token| {
            (
                token.kind(),
                token.range(),
                token.flags(),
                token.text().unwrap(),
            )
        })
        .collect()
}

fn assert_default_invariants(parsed: &CanonicalSourceRuleSnapshot, rule: RuleId) {
    assert_eq!(parsed.rule, rule);
    assert!(parsed.diagnostics.is_empty(), "{rule:?}");
    assert!(parsed.stats.parser_steps <= ParseConfig::default().limits.fuel);
    assert!(
        parsed.stats.events_emitted <= ParseConfig::default().limits.max_events as u64,
        "{rule:?}"
    );
    if parsed.matched {
        assert!(parsed.is_strictly_clean(), "{rule:?}");
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
            .unwrap_or_else(|error| panic!("{rule:?} is not lossless: {error:?}"));
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            parsed.source.text(parsed.consumed).unwrap(),
            "{rule:?}"
        );
    } else {
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{rule:?} did not restore its failed candidate"
        );
    }
}

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 64,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2b_540_2a0),
    ..ProptestConfig::default()
  })]

  #[test]
  fn every_phase_2h_direct_rule_is_total_lossless_and_non_diagnostic(
    characters in proptest::collection::vec(any::<char>(), 0..48),
  ) {
    let text = characters.into_iter().collect::<String>();
    for rule in PHASE_2H_RULES {
      let parsed = parse(source(&text), *rule, ParseConfig::default());
      assert_default_invariants(&parsed, *rule);
    }
  }
}

#[test]
fn every_phase_2h_nomatch_is_transactional() {
    for rule in PHASE_2H_RULES {
        let parsed = parse(source("\u{a0}"), *rule, ParseConfig::default());
        assert!(!parsed.matched, "{rule:?}");
        assert_default_invariants(&parsed, *rule);
    }
}

#[test]
fn direct_phase_2h_rules_respect_hard_event_and_fuel_limits() {
    let config = ParseConfig {
        limits: ParseLimits {
            fuel: 64,
            max_events: 16,
            ..ParseLimits::default()
        },
    };
    for (rule, input) in [
        (rules::TABLE_TOP, format!("╭{}\n", "─".repeat(8_192))),
        (rules::ROW_SEPARATOR, "─".repeat(8_192)),
        (
            rules::TABLE_SEPARATOR,
            format!(
                "{}|{}",
                " \t\u{00a0}\u{2009}".repeat(8_192),
                " \t".repeat(8_192)
            ),
        ),
        (
            rules::EMPTY_MAP,
            format!("{{{}:{} }}", "\n ".repeat(8_192), "\t\r".repeat(8_192)),
        ),
    ] {
        let parsed = parse(source(&input), rule, config);
        assert!(parsed.stats.parser_steps <= config.limits.fuel, "{rule:?}");
        assert!(
            parsed.stats.events_emitted <= config.limits.max_events as u64,
            "{rule:?}"
        );
    }
}

#[test]
fn contiguous_and_piece_backed_phase_2h_sources_agree() {
    let cases: &[(RuleId, &[&str])] = &[
        (rules::TABLE_SEPARATOR, &[" ", "|", " "]),
        (rules::TABLE_TOP, &["╭", "─", "─", "\r", "\n"]),
        (rules::EMPTY_MAP, &["{", "\n", ":", "\n", "}"]),
        (rules::EMPTY_SET, &["{", "_", "}"]),
        (rules::ROW_SEPARATOR, &["┼", "─", "┘"]),
    ];

    for (rule, parts) in cases {
        let text = parts.concat();
        let contiguous = parse(source(&text), *rule, ParseConfig::default());
        let piece_backed = parse(piece_source(parts), *rule, ParseConfig::default());
        assert_default_invariants(&contiguous, *rule);
        assert_default_invariants(&piece_backed, *rule);
        assert_eq!(
            piece_backed.matched, contiguous.matched,
            "{rule:?} on {text:?}"
        );
        assert_eq!(
            piece_backed.consumed, contiguous.consumed,
            "{rule:?} on {text:?}"
        );
        assert_eq!(
            compact_debug_tree(&piece_backed.syntax()),
            compact_debug_tree(&contiguous.syntax()),
            "{rule:?} on {text:?}"
        );
        assert_eq!(token_signature(&piece_backed), token_signature(&contiguous));
        assert_eq!(
            normalize_diagnostics(
                &piece_backed.diagnostics,
                piece_backed.source.revision(),
                &piece_backed.nodes,
            ),
            normalize_diagnostics(
                &contiguous.diagnostics,
                contiguous.source.revision(),
                &contiguous.nodes,
            ),
            "{rule:?} on {text:?}"
        );
    }
}

fn measurements(rule: RuleId, inputs: impl IntoIterator<Item = String>) -> Vec<(u64, u64)> {
    inputs
        .into_iter()
        .map(|input| {
            let parsed = parse(source(&input), rule, ParseConfig::default());
            assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
            (parsed.stats.parser_steps, parsed.stats.events_emitted)
        })
        .collect()
}

fn assert_linear(measurements: &[(u64, u64)]) {
    for pair in measurements.windows(2) {
        let (small_steps, small_events) = pair[0];
        let (large_steps, large_events) = pair[1];
        assert!(
            large_steps <= small_steps.saturating_mul(2).saturating_add(256),
            "parser steps were not linear: {measurements:?}"
        );
        assert!(
            large_events <= small_events.saturating_mul(2).saturating_add(256),
            "parser events were not linear: {measurements:?}"
        );
    }
}

#[test]
fn border_and_horizontal_trivia_parsing_grow_linearly() {
    let sizes = [32_usize, 64, 128, 256];
    assert_linear(&measurements(
        rules::TABLE_TOP,
        sizes
            .into_iter()
            .map(|size| format!("╭{}\n", "─".repeat(size))),
    ));
    assert_linear(&measurements(
        rules::ROW_SEPARATOR,
        sizes.into_iter().map(|size| "─".repeat(size)),
    ));
    for inputs in [
        sizes.map(|size| format!("─{}", " ".repeat(size))),
        sizes.map(|size| format!("─{}", "\t".repeat(size))),
        sizes.map(|size| format!("─{}", "\u{00a0}\u{2009}".repeat(size))),
        sizes.map(|size| format!("─{}x", " ".repeat(size))),
        sizes.map(|size| {
            let spaces = " ".repeat(size);
            format!("─{spaces}|{spaces}")
        }),
    ] {
        assert_linear(&measurements(rules::ROW_SEPARATOR, inputs));
    }
    assert_linear(&measurements(
        rules::TABLE_SEPARATOR,
        sizes.into_iter().map(|size| {
            let surrounding = " \t\u{00a0}\u{2009}".repeat(size);
            format!("{surrounding}|{surrounding}")
        }),
    ));
}
