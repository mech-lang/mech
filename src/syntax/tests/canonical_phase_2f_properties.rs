use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2f_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, TextRange, TextSize,
    TextSnapshot, compact_debug_tree, reconstruct_source_range, validate_lossless_range,
};
use proptest::prelude::*;

const PHASE_2F_RULES: &[RuleId; 21] = &[
    rules::SOURCE_IMPORT_TAIL,
    rules::SOURCE_PATH_COMPONENT_TOKEN,
    rules::SOURCE_PATH_COMPONENT,
    rules::SOURCE_MEC_PATH,
    rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX,
    rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
    rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER,
    rules::BARE_SOURCE_IMPORT_SPECIFIER,
    rules::URI_SCHEME_PART,
    rules::SOURCE_IMPORT_URI_SCHEME,
    rules::URI_SOURCE_IMPORT_SPECIFIER,
    rules::SOURCE_IMPORT_SPECIFIER,
    rules::IMPORT_DECLARATION,
    rules::EXPORT_DECLARATION,
    rules::CONTEXT_DECLARATION,
    rules::CONTEXT_BASE_CONTEXT,
    rules::CONTEXT_BASE_RESOURCE_URI,
    rules::CONTEXT_CAPABILITY_DECLARATION,
    rules::CONTEXT_CAPABILITY_PATH_TOKEN,
    rules::CONTEXT_CAPABILITY_PATH,
    rules::CONTEXT_CAPABILITY_SCOPE,
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(937), Revision(0), text).unwrap()
}

fn piece_source(parts: &[&str]) -> TextSnapshot {
    let mut snapshot = source("");
    for part in parts {
        snapshot = snapshot.append((*part).to_owned()).unwrap();
    }
    snapshot
}

fn parse(source: TextSnapshot, rule: RuleId, config: ParseConfig) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2f_rule_for_test(source, rule, config).unwrap()
}

fn token_fingerprint(parsed: &CanonicalSourceRuleSnapshot) -> Vec<(SyntaxKind, String, TextRange)> {
    parsed
        .syntax()
        .tokens()
        .into_iter()
        .map(|token| (token.kind(), token.text().unwrap(), token.range()))
        .collect()
}

fn assert_piece_equivalent(rule: RuleId, parts: &[&str], expected_lowered_text: Option<&str>) {
    let joined = parts.concat();
    let contiguous = parse(source(&joined), rule, ParseConfig::default());
    let piece_backed = parse(piece_source(parts), rule, ParseConfig::default());
    assert_eq!(contiguous.outcome, piece_backed.outcome, "{rule:?}");
    assert_eq!(contiguous.consumed, piece_backed.consumed, "{rule:?}");
    assert_eq!(
        compact_debug_tree(&contiguous.syntax()),
        compact_debug_tree(&piece_backed.syntax()),
        "{rule:?}"
    );
    assert_eq!(
        token_fingerprint(&contiguous),
        token_fingerprint(&piece_backed),
        "{rule:?}"
    );
    assert_eq!(
        contiguous.diagnostics.len(),
        piece_backed.diagnostics.len(),
        "{rule:?}"
    );
    assert!(contiguous.diagnostics.is_empty(), "{rule:?}");
    assert!(piece_backed.diagnostics.is_empty(), "{rule:?}");

    if let Some(expected) = expected_lowered_text {
        use mech_syntax::document::{
            AstNode, ImportDeclarationSyntax, lower_legacy_import_declaration,
        };

        let contiguous_declaration = ImportDeclarationSyntax::cast(
            contiguous
                .syntax()
                .children()
                .find(|child| child.kind() == SyntaxKind::ImportDeclaration)
                .unwrap(),
        )
        .unwrap();
        let piece_backed_declaration = ImportDeclarationSyntax::cast(
            piece_backed
                .syntax()
                .children()
                .find(|child| child.kind() == SyntaxKind::ImportDeclaration)
                .unwrap(),
        )
        .unwrap();
        let contiguous_lowered = lower_legacy_import_declaration(&contiguous_declaration).unwrap();
        let piece_backed_lowered =
            lower_legacy_import_declaration(&piece_backed_declaration).unwrap();
        assert_eq!(contiguous_lowered, piece_backed_lowered);
        assert_eq!(contiguous_lowered.specifier.to_string(), expected);
    }
}

fn assert_invariants(parsed: &CanonicalSourceRuleSnapshot, config: ParseConfig) {
    assert!(parsed.stats.parser_steps <= config.limits.fuel);
    assert!(parsed.stats.events_emitted <= u64::from(config.limits.max_events));
    match parsed.outcome {
        CanonicalRuleOutcome::NoMatch => {
            assert!(!parsed.matched);
            assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
            if config == ParseConfig::default() {
                assert!(parsed.diagnostics.is_empty());
            }
        }
        CanonicalRuleOutcome::Matched | CanonicalRuleOutcome::Committed => {
            assert!(parsed.matched);
            assert!(parsed.source.full_range().contains_range(parsed.consumed));
            validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
            assert_eq!(
                reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
                parsed.source.text(parsed.consumed).unwrap(),
            );
        }
    }
    for diagnostic in parsed.diagnostics.iter() {
        let range = diagnostic
            .primary
            .resolve(parsed.source.revision(), &parsed.nodes)
            .unwrap();
        assert!(parsed.source.full_range().contains_range(range));
    }
}

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 64,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2f_539_303),
    ..ProptestConfig::default()
  })]

  #[test]
  fn every_phase_2f_direct_rule_is_total_lossless_and_bounded(
    characters in proptest::collection::vec(any::<char>(), 0..48),
  ) {
    let input = characters.into_iter().collect::<String>();
    for rule in PHASE_2F_RULES {
      let config = ParseConfig::default();
      assert_invariants(&parse(source(&input), *rule, config), config);
    }
  }
}

#[test]
fn zero_width_wildcard_suffix_and_piece_backed_inputs_are_deterministic() {
    for input in ["", "/", "/x"] {
        let parsed = parse(
            source(input),
            rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX,
            ParseConfig::default(),
        );
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
        assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
    }

    let cases: &[(RuleId, &[&str], Option<&str>)] = &[
        (rules::SOURCE_MEC_PATH, &["foo", ".", "mec"], None),
        (
            rules::BARE_SOURCE_IMPORT_SPECIFIER,
            &["foo.m", "ec", "/", "*"],
            None,
        ),
        (
            rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
            &[".", ".", "/", "lib/", "dep.mec"],
            None,
        ),
        (
            rules::IMPORT_DECLARATION,
            &["+", ">", "\u{2009}", "dep.mec"],
            Some("dep.mec"),
        ),
        (
            rules::URI_SOURCE_IMPORT_SPECIFIER,
            &[
                "https",
                ":",
                "/",
                "/",
                "example.com/dep",
                "\u{00a0}",
                "\u{2009}",
            ],
            None,
        ),
        (
            rules::IMPORT_DECLARATION,
            &[
                "+",
                ">",
                " ",
                "https",
                ":",
                "/",
                "/",
                "example.com/dep",
                "\u{00a0}",
                "\u{2009}",
            ],
            Some("https://example.com/dep"),
        ),
        (rules::EXPORT_DECLARATION, &["<", "+", "\n", "value"], None),
        (
            rules::CONTEXT_DECLARATION,
            &["@", "ui", " := ", "fs", "://", "workspace"],
            None,
        ),
        (
            rules::CONTEXT_DECLARATION,
            &[
                "@",
                "users",
                " := ",
                "@",
                "main",
                "{:read(users",
                "/",
                "*",
                ")}",
            ],
            None,
        ),
        (rules::CONTEXT_CAPABILITY_PATH, &["users", "/", "*"], None),
    ];
    for (rule, parts, expected_lowered_text) in cases {
        assert_piece_equivalent(*rule, parts, *expected_lowered_text);
    }
}

#[test]
fn fuel_and_event_limits_remain_hard_for_linear_inputs() {
    let config = ParseConfig {
        limits: ParseLimits {
            fuel: 64,
            max_events: 32,
            ..ParseLimits::default()
        },
    };
    for (rule, input) in [
        (
            rules::SOURCE_MEC_PATH,
            format!("{}tail.mec", "a/".repeat(8_192)),
        ),
        (
            rules::URI_SOURCE_IMPORT_SPECIFIER,
            format!("x://{}", "a".repeat(16_384)),
        ),
        (rules::CONTEXT_CAPABILITY_PATH, "users/".repeat(8_192)),
        (
            rules::CONTEXT_DECLARATION,
            format!("@ctx := @base{{{}", ":op(*)".repeat(8_192)),
        ),
    ] {
        assert_invariants(&parse(source(&input), rule, config), config);
    }
}

#[test]
fn linear_inputs_keep_parser_steps_and_events_linear() {
    let sizes = [32usize, 64, 128, 256];
    for (rule, inputs) in [
        (
            rules::SOURCE_MEC_PATH,
            sizes.map(|count| format!("{}tail.mec", "a/".repeat(count))),
        ),
        (
            rules::URI_SOURCE_IMPORT_SPECIFIER,
            sizes.map(|count| format!("x://{}", "a".repeat(count))),
        ),
        (
            rules::CONTEXT_CAPABILITY_PATH,
            sizes.map(|count| format!("{}read", "users/".repeat(count))),
        ),
        (
            rules::CONTEXT_DECLARATION,
            sizes.map(|count| {
                format!(
                    "@ctx := @base{{{}}}",
                    (1..=count)
                        .map(|index| format!(":op{index}(*)"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }),
        ),
    ] {
        let mut previous_steps = 0;
        let mut previous_events = 0;
        for (size, input) in sizes.into_iter().zip(inputs) {
            let parsed = parse(source(&input), rule, ParseConfig::default());
            assert!(parsed.is_strictly_clean(), "{rule:?}, size {size}");
            assert!(
                parsed.stats.parser_steps >= previous_steps,
                "{rule:?}, size {size}"
            );
            assert!(
                parsed.stats.events_emitted >= previous_events,
                "{rule:?}, size {size}"
            );
            assert!(parsed.stats.parser_steps <= (size as u64 * 32) + 128);
            assert!(parsed.stats.events_emitted <= (size as u64 * 32) + 128);
            previous_steps = parsed.stats.parser_steps;
            previous_events = parsed.stats.events_emitted;
        }
    }
}
