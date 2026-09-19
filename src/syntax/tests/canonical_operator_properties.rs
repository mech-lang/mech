use mech_syntax::document::ast::OperatorSyntax;
use mech_syntax::document::parser::canonical::parse_canonical_phase_2d_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, TextRange,
    TextSize, TextSnapshot, TokenFlags, compact_debug_tree, normalize_diagnostics,
    reconstruct_source_range, validate_lossless_range,
};
use proptest::prelude::*;

const PHASE_2D_RULES: &[RuleId] = &[
    rules::ADD_SUB_OPERATOR,
    rules::MUL_DIV_OPERATOR,
    rules::POWER_OPERATOR,
    rules::MATRIX_OPERATOR,
    rules::RANGE_OPERATOR,
    rules::COMPARISON_OPERATOR,
    rules::LOGIC_OPERATOR,
    rules::TABLE_OPERATOR,
    rules::SET_OPERATOR,
    rules::ADD,
    rules::SUBTRACT,
    rules::RAW_SUBTRACT,
    rules::SPACED_SUBTRACT,
    rules::MULTIPLY,
    rules::DIVIDE,
    rules::MODULUS,
    rules::POWER,
    rules::MATRIX_MULTIPLY,
    rules::MATRIX_SOLVE,
    rules::DOT_PRODUCT,
    rules::CROSS_PRODUCT,
    rules::TRANSPOSE,
    rules::RANGE_INCLUSIVE,
    rules::RANGE_EXCLUSIVE,
    rules::NOT_EQUAL,
    rules::EQUAL_TO,
    rules::STRICT_NOT_EQUAL,
    rules::STRICT_EQUAL,
    rules::GREATER_THAN,
    rules::LESS_THAN,
    rules::GREATER_THAN_EQUAL,
    rules::LESS_THAN_EQUAL,
    rules::OR,
    rules::AND,
    rules::NOT,
    rules::XOR,
    rules::JOIN,
    rules::LEFT_JOIN,
    rules::RIGHT_JOIN,
    rules::FULL_JOIN,
    rules::LEFT_SEMI_JOIN,
    rules::LEFT_ANTI_JOIN,
    rules::UNION_OP,
    rules::INTERSECTION,
    rules::DIFFERENCE,
    rules::COMPLEMENT,
    rules::SUBSET,
    rules::SUPERSET,
    rules::PROPER_SUBSET,
    rules::PROPER_SUPERSET,
    rules::ELEMENT_OF,
    rules::NOT_ELEMENT_OF,
    rules::SYMMETRIC_DIFFERENCE,
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(925), Revision(0), text).unwrap()
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
    parse_canonical_phase_2d_rule_for_test(source, rule, config)
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2D direct rule"))
}

fn operator_semantic(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
) -> Option<mech_syntax::document::ast::CanonicalOperator> {
    fn find(syntax: &mech_syntax::document::SyntaxNode) -> Option<OperatorSyntax> {
        OperatorSyntax::cast(syntax.clone())
            .or_else(|| syntax.children().find_map(|child| find(&child)))
    }
    find(&parsed.syntax()).and_then(|operator| operator.semantic())
}

fn contains_node(syntax: &mech_syntax::document::SyntaxNode, kind: SyntaxKind) -> bool {
    syntax.kind() == kind || syntax.children().any(|child| contains_node(&child, kind))
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

fn assert_default_invariants(
    parsed: &mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot,
    rule: RuleId,
) {
    assert_eq!(parsed.rule, rule);
    assert!(parsed.diagnostics.is_empty(), "{rule:?}");
    assert!(parsed.stats.parser_steps <= ParseConfig::default().limits.fuel);
    assert!(
        parsed.stats.events_emitted <= u64::from(ParseConfig::default().limits.max_events),
        "{rule:?}"
    );
    if parsed.matched {
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
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2d_540_263),
    ..ProptestConfig::default()
  })]

  #[test]
  fn every_phase_2d_direct_rule_is_total_lossless_and_non_diagnostic(
    characters in proptest::collection::vec(any::<char>(), 0..48),
  ) {
    let text = characters.into_iter().collect::<String>();
    for rule in PHASE_2D_RULES {
      let parsed = parse(source(&text), *rule, ParseConfig::default());
      assert_default_invariants(&parsed, *rule);
    }
  }
}

#[test]
fn every_phase_2d_nomatch_is_transactional() {
    for rule in PHASE_2D_RULES {
        let parsed = parse(source("\n"), *rule, ParseConfig::default());
        assert!(!parsed.matched, "{rule:?}");
        assert_default_invariants(&parsed, *rule);
    }
}

#[test]
fn direct_operators_respect_hard_event_and_fuel_limits() {
    let config = ParseConfig {
        limits: ParseLimits {
            fuel: 64,
            max_events: 16,
            ..ParseLimits::default()
        },
    };
    for (rule, input) in [
        (
            rules::ADD,
            format!("{}+{}", " ".repeat(8_192), " ".repeat(8_192)),
        ),
        (
            rules::STRICT_EQUAL,
            format!("{}==={}", " ".repeat(8_192), " ".repeat(8_192)),
        ),
        (
            rules::JOIN,
            format!("{}⋈{}", " ".repeat(8_192), " ".repeat(8_192)),
        ),
        (
            rules::SYMMETRIC_DIFFERENCE,
            format!("{}Δ{}", " ".repeat(8_192), " ".repeat(8_192)),
        ),
    ] {
        let parsed = parse(source(&input), rule, config);
        assert!(parsed.stats.parser_steps <= config.limits.fuel, "{rule:?}");
        assert!(
            parsed.stats.events_emitted <= u64::from(config.limits.max_events),
            "{rule:?}"
        );
    }
}

#[test]
fn contiguous_and_piece_backed_operator_sources_agree() {
    let cases: &[(RuleId, &[&str])] = &[
        (rules::MATRIX_OPERATOR, &["*", "*"]),
        (rules::RANGE_OPERATOR, &[".", ".="]),
        (rules::COMPARISON_OPERATOR, &["!", "=="]),
        (rules::COMPARISON_OPERATOR, &["=", "=="]),
        (rules::COMPARISON_OPERATOR, &[">", "="]),
        (rules::COMPARISON_OPERATOR, &["<", "="]),
        (rules::LESS_THAN, &["<", "-"]),
        (rules::DIVIDE, &["/", "/"]),
        (rules::TABLE_OPERATOR, &[" ", "⋈", " "]),
        (rules::SET_OPERATOR, &["\t", "Δ", "\u{2009}"]),
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
        assert_eq!(
            operator_semantic(&piece_backed),
            operator_semantic(&contiguous)
        );
    }
}

#[test]
fn aggregate_selection_is_deterministic_and_ordered() {
    for (rule, input, selected) in [
        (
            rules::ADD_SUB_OPERATOR,
            " - ",
            SyntaxKind::SubtractOperation,
        ),
        (
            rules::MATRIX_OPERATOR,
            "**",
            SyntaxKind::MatrixMultiplyOperation,
        ),
        (
            rules::RANGE_OPERATOR,
            "..=",
            SyntaxKind::RangeInclusiveOperation,
        ),
        (
            rules::COMPARISON_OPERATOR,
            "===",
            SyntaxKind::StrictEqualOperation,
        ),
        (
            rules::COMPARISON_OPERATOR,
            "!==",
            SyntaxKind::StrictNotEqualOperation,
        ),
        (
            rules::COMPARISON_OPERATOR,
            ">=",
            SyntaxKind::GreaterThanEqualOperation,
        ),
        (
            rules::COMPARISON_OPERATOR,
            "<=",
            SyntaxKind::LessThanEqualOperation,
        ),
    ] {
        let first = parse(source(input), rule, ParseConfig::default());
        let second = parse(source(input), rule, ParseConfig::default());
        assert_default_invariants(&first, rule);
        assert_default_invariants(&second, rule);
        assert_eq!(
            compact_debug_tree(&first.syntax()),
            compact_debug_tree(&second.syntax())
        );
        assert!(
            contains_node(&first.syntax(), selected),
            "{rule:?} did not choose {selected:?} for {input:?}"
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
fn surrounding_operator_whitespace_grows_linearly() {
    let sizes = [32_usize, 64, 128, 256];
    for (rule, operator) in [
        (rules::ADD, "+"),
        (rules::STRICT_EQUAL, "==="),
        (rules::JOIN, "⋈"),
        (rules::SYMMETRIC_DIFFERENCE, "Δ"),
    ] {
        assert_linear(&measurements(
            rule,
            sizes
                .into_iter()
                .map(|size| format!("{}{}{}", " ".repeat(size), operator, " ".repeat(size))),
        ));
    }
}
