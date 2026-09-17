use mech_syntax::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, RecoveryAction, Revision, RuleId, SyntaxKind, TextRange,
    TextSize, TextSnapshot, TokenFlags, compact_debug_tree, normalize_diagnostics,
    reconstruct_source_range, validate_lossless, validate_lossless_range,
};
use proptest::prelude::*;

const PHASE_2C_RULES: &[RuleId] = &[
    rules::EMPTY,
    rules::ATOM,
    rules::STRING,
    rules::UTF8_STRING,
    rules::RAW_STRING,
    rules::BOOLEAN,
    rules::TRUE_LITERAL,
    rules::FALSE_LITERAL,
    rules::NUMBER,
    rules::COMPLEX_NUMBER,
    rules::REAL_NUMBER,
    rules::UNTYPED_REAL_NUMBER,
    rules::RATIONAL_LITERAL,
    rules::SCIENTIFIC_LITERAL,
    rules::FLOAT_DECIMAL_START,
    rules::FLOAT_FULL,
    rules::FLOAT_LITERAL,
    rules::INTEGER_LITERAL,
    rules::TYPED_INTEGER,
    rules::UNTYPED_INTEGER,
    rules::DECIMAL_LITERAL,
    rules::HEXADECIMAL_LITERAL,
    rules::OCTAL_LITERAL,
    rules::BINARY_LITERAL,
    rules::CONTEXT_ADDRESS_PATH_TOKEN,
    rules::CONTEXT_ADDRESS_PATH,
    rules::PREFIXED_CONTEXT_PATH,
    rules::KIND_ANY,
    rules::KIND_EMPTY,
    rules::KIND_ATOM,
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(922), Revision(0), text).unwrap()
}

fn piece_source(parts: &[&str]) -> TextSnapshot {
    let mut source = source("");
    for part in parts {
        source = source.append((*part).to_owned()).unwrap();
    }
    assert_eq!(source.piece_count(), parts.len());
    source
}

fn parse(
    source: TextSnapshot,
    rule: RuleId,
    config: ParseConfig,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2c_rule_for_test(source, rule, config)
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2C direct rule"))
}

fn assert_diagnostic_ranges_are_bounded(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
) {
    let source_range = parsed.source.full_range();
    for diagnostic in parsed.diagnostics.iter() {
        let primary = diagnostic
            .primary
            .resolve(parsed.source.revision(), &parsed.nodes);
        assert!(primary.is_some(), "unresolvable diagnostic: {diagnostic:?}");
        assert!(source_range.contains_range(primary.unwrap()));
        for label in &diagnostic.labels {
            let range = label
                .anchor
                .resolve(parsed.source.revision(), &parsed.nodes);
            assert!(range.is_some(), "unresolvable label: {label:?}");
            assert!(source_range.contains_range(range.unwrap()));
        }
        for fix in &diagnostic.fixes {
            for edit in &fix.edits {
                assert!(source_range.contains_range(edit.delete));
            }
        }
        match diagnostic.recovery.as_ref() {
            Some(RecoveryAction::Insert { at, .. }) | Some(RecoveryAction::Abandon { at, .. }) => {
                assert!(source_range.contains_inclusive(*at));
            }
            Some(RecoveryAction::Skip { range })
            | Some(RecoveryAction::ResourceLimit { range }) => {
                assert!(source_range.contains_range(*range));
            }
            None => {}
        }
    }
}

fn assert_snapshot_invariants(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
    rule: RuleId,
    config: ParseConfig,
) {
    assert_eq!(parsed.rule, rule);
    assert_eq!(parsed.syntax().kind(), SyntaxKind::CanonicalFragment);
    assert_eq!(parsed.consumed.start, TextSize::ZERO);
    assert!(parsed.source.full_range().contains_range(parsed.consumed));
    assert!(parsed.stats.parser_steps <= config.limits.fuel);
    assert!(parsed.stats.events_emitted <= u64::from(config.limits.max_events));
    if parsed.root.text_len == parsed.consumed.len() {
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap_or_else(
        |error| {
            panic!(
                "{rule:?} violated losslessness with consumed {:?}, source {:?}, stats {:?}: {error:?}",
                parsed.consumed,
                parsed.source.byte_len(),
                parsed.stats,
            )
        },
    );
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            parsed.source.text(parsed.consumed).unwrap()
        );
    } else {
        // Resource-limit recovery preserves the complete physical source in an
        // error envelope after the direct production's consumed prefix.
        assert_eq!(parsed.root.text_len, parsed.source.byte_len());
        validate_lossless(&parsed.root, &parsed.source).unwrap();
    }
    assert_diagnostic_ranges_are_bounded(parsed);
}

fn token_signature(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
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

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 64,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2c_540_210),
    ..ProptestConfig::default()
  })]

  #[test]
  fn every_phase_2c_direct_rule_is_total_lossless_and_bounded(
    characters in proptest::collection::vec(any::<char>(), 0..48),
  ) {
    let text = characters.into_iter().collect::<String>();
    for rule in PHASE_2C_RULES {
      let config = ParseConfig::default();
      let parsed = parse(source(&text), *rule, config);
      assert_snapshot_invariants(&parsed, *rule, config);
    }
  }
}

#[test]
fn all_phase_2c_direct_rules_restore_a_clean_nomatch() {
    for rule in PHASE_2C_RULES {
        let parsed = parse(source("!"), *rule, ParseConfig::default());
        assert!(!parsed.matched, "{rule:?}");
        assert!(parsed.diagnostics.is_empty(), "{rule:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{rule:?}"
        );
        assert_snapshot_invariants(&parsed, *rule, ParseConfig::default());
    }
}

#[test]
fn direct_rules_respect_hard_fuel_and_event_limits() {
    let config = ParseConfig {
        limits: ParseLimits {
            fuel: 64,
            max_events: 16,
            ..ParseLimits::default()
        },
    };
    let cases = [
        (rules::UTF8_STRING, format!("\"{}\"", "x".repeat(8_192))),
        (rules::UNTYPED_INTEGER, "1".repeat(8_192)),
        (rules::CONTEXT_ADDRESS_PATH, "a/".repeat(8_192)),
    ];

    for (rule, input) in cases {
        let parsed = parse(source(&input), rule, config);
        assert_snapshot_invariants(&parsed, rule, config);
    }
}

#[test]
fn contiguous_and_piece_backed_phase_2c_sources_are_equivalent() {
    let cases: &[(RuleId, &[&str])] = &[
        (rules::UTF8_STRING, &["\"", "e", "\u{301}", "\""]),
        (rules::RAW_STRING, &["\"", "\"", "\"raw", "\"", "\""]),
        (rules::RAW_STRING, &["\"\"\"raw\"", "\"", "\""]),
        (rules::HEXADECIMAL_LITERAL, &["0", "xG_"]),
        (rules::UNTYPED_INTEGER, &["1_", "000"]),
        (rules::TYPED_INTEGER, &["1", "u", "8"]),
        (rules::ATOM, &[":", "💡"]),
        (
            rules::PREFIXED_CONTEXT_PATH,
            &["@c", "tx/pa", "th/to.value_1"],
        ),
        (rules::CONTEXT_ADDRESS_PATH, &["path/", "to.", "value_1"]),
    ];

    for (rule, parts) in cases {
        let text = parts.concat();
        let contiguous = parse(source(&text), *rule, ParseConfig::default());
        let piece_backed = parse(piece_source(parts), *rule, ParseConfig::default());
        assert_snapshot_invariants(&contiguous, *rule, ParseConfig::default());
        assert_snapshot_invariants(&piece_backed, *rule, ParseConfig::default());
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
            "{rule:?} on {text:?}",
        );
        assert_eq!(
            token_signature(&piece_backed),
            token_signature(&contiguous),
            "{rule:?} on {text:?}",
        );
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
            "{rule:?} on {text:?}",
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
        let (smaller_steps, smaller_events) = pair[0];
        let (larger_steps, larger_events) = pair[1];
        assert!(
            larger_steps <= smaller_steps.saturating_mul(2).saturating_add(256),
            "parser steps were not linear: {measurements:?}",
        );
        assert!(
            larger_events <= smaller_events.saturating_mul(2).saturating_add(256),
            "parser events were not linear: {measurements:?}",
        );
    }
}

#[test]
fn long_closed_literals_and_paths_grow_linearly() {
    let sizes = [32_usize, 64, 128, 256];
    assert_linear(&measurements(
        rules::UTF8_STRING,
        sizes
            .into_iter()
            .map(|size| format!("\"{}\"", "x".repeat(size))),
    ));
    assert_linear(&measurements(
        rules::UNTYPED_INTEGER,
        sizes.into_iter().map(|size| "1".repeat(size)),
    ));
    assert_linear(&measurements(
        rules::CONTEXT_ADDRESS_PATH,
        sizes.into_iter().map(|size| "a/".repeat(size)),
    ));
}
