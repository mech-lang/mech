use mech_syntax::document::ast::ModuleImportSyntax;
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2e_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, NodeFlags, ParseConfig, ParseLimits, RecoveryAction, Revision, RuleId,
    SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags, compact_debug_tree,
    lower_legacy_module_import, normalize_diagnostics, reconstruct_source_range, validate_lossless,
    validate_lossless_range,
};
use proptest::prelude::*;

const PHASE_2E_RULES: &[RuleId] = &[
    rules::MODULE_IMPORT_NAME_SEGMENT,
    rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
    rules::MODULE_IMPORT_PATH_SEGMENT,
    rules::MODULE_IMPORT_PATH,
    rules::MODULE_IMPORT_ALIAS_SEGMENT,
    rules::MODULE_IMPORT_ALIAS_PATH,
    rules::MODULE_IMPORT_VALUE_ALIAS,
    rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
    rules::MODULE_IMPORT_CONTEXT_ALIAS,
    rules::MODULE_IMPORT_ALIAS,
    rules::MODULE_ROOT,
    rules::IMPORT_ALIAS_OPERATOR,
    rules::IMPORT_GROUP_SEPARATOR,
    rules::IMPORT_GROUP_ITEM,
    rules::IMPORT_GROUP_ITEMS,
    rules::ALIASED_ITEM_IMPORT,
    rules::MODULE_SUFFIX_IMPORT,
    rules::MODULE_ONLY_IMPORT,
    rules::MODULE_IMPORT,
];

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(930), Revision(0), text).unwrap()
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
    parse_canonical_phase_2e_rule_for_test(source, rule, config)
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2E direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn assert_diagnostic_ranges_are_bounded(parsed: &CanonicalSourceRuleSnapshot) {
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
    parsed: &CanonicalSourceRuleSnapshot,
    rule: RuleId,
    config: ParseConfig,
) {
    assert_eq!(parsed.rule, rule);
    assert_eq!(parsed.syntax().kind(), SyntaxKind::CanonicalFragment);
    assert_eq!(parsed.consumed.start, TextSize::ZERO);
    assert!(parsed.source.full_range().contains_range(parsed.consumed));
    assert!(parsed.stats.parser_steps <= config.limits.fuel, "{rule:?}");
    assert!(
        parsed.stats.events_emitted <= u64::from(config.limits.max_events),
        "{rule:?}"
    );

    match parsed.outcome {
        CanonicalRuleOutcome::NoMatch => {
            assert!(!parsed.matched, "{rule:?}");
            assert_eq!(
                parsed.consumed,
                TextRange::empty(TextSize::ZERO),
                "{rule:?}"
            );
            if config == ParseConfig::default() {
                assert!(parsed.diagnostics.is_empty(), "{rule:?}");
            } else {
                assert!(parsed.diagnostics.iter().all(|diagnostic| {
                    matches!(
                        diagnostic.recovery,
                        Some(RecoveryAction::ResourceLimit { .. })
                    )
                }));
            }
        }
        CanonicalRuleOutcome::Matched | CanonicalRuleOutcome::Committed => {
            assert!(parsed.matched, "{rule:?}");
        }
    }

    if parsed.outcome == CanonicalRuleOutcome::Committed {
        assert!(!parsed.is_strictly_clean(), "{rule:?}");
        assert!(
            !parsed.diagnostics.is_empty()
                || parsed.root.flags.intersects(
                    NodeFlags::ERROR
                        | NodeFlags::MISSING
                        | NodeFlags::CONTAINS_ERROR
                        | NodeFlags::CONTAINS_MISSING,
                ),
            "{rule:?} committed without a diagnostic or structural error marker"
        );
    }

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
            parsed.source.text(parsed.consumed).unwrap(),
        );
    } else {
        assert_eq!(parsed.root.text_len, parsed.source.byte_len());
        validate_lossless(&parsed.root, &parsed.source).unwrap();
    }
    assert_diagnostic_ranges_are_bounded(parsed);
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

fn lowered_module_import(parsed: &CanonicalSourceRuleSnapshot) -> mech_core::nodes::ModuleImport {
    let node = find_node(&parsed.syntax(), SyntaxKind::ModuleImport).unwrap();
    let syntax = ModuleImportSyntax::cast(node).unwrap();
    lower_legacy_module_import(&syntax).unwrap()
}

proptest! {
  #![proptest_config(ProptestConfig {
    cases: 64,
    rng_seed: proptest::test_runner::RngSeed::Fixed(0x2e_539_282),
    ..ProptestConfig::default()
  })]

  #[test]
  fn every_phase_2e_direct_rule_is_total_lossless_and_bounded(
    characters in proptest::collection::vec(any::<char>(), 0..48),
  ) {
    let text = characters.into_iter().collect::<String>();
    for rule in PHASE_2E_RULES {
      let config = ParseConfig::default();
      let parsed = parse(source(&text), *rule, config);
      assert_snapshot_invariants(&parsed, *rule, config);
    }
  }
}

#[test]
fn all_phase_2e_direct_rules_restore_a_clean_nomatch() {
    assert_eq!(PHASE_2E_RULES.len(), 19);
    for rule in PHASE_2E_RULES {
        let parsed = parse(source("$"), *rule, ParseConfig::default());
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{rule:?}");
        assert_snapshot_invariants(&parsed, *rule, ParseConfig::default());
    }
}

#[test]
fn direct_import_rules_respect_hard_fuel_and_event_limits() {
    let config = ParseConfig {
        limits: ParseLimits {
            fuel: 64,
            max_events: 16,
            ..ParseLimits::default()
        },
    };
    for (rule, input) in [
        (rules::MODULE_IMPORT_PATH, "a/".repeat(8_192)),
        (rules::MODULE_IMPORT_ALIAS_PATH, "a/".repeat(8_192)),
        (
            rules::IMPORT_GROUP_ITEMS,
            format!("{}a", "a,".repeat(8_192)),
        ),
        (
            rules::MODULE_IMPORT,
            format!("+> math/{{{}a", "a,".repeat(8_192)),
        ),
    ] {
        let parsed = parse(source(&input), rule, config);
        assert_snapshot_invariants(&parsed, rule, config);
    }
}

#[test]
fn contiguous_and_piece_backed_module_import_sources_agree() {
    let cases: &[(RuleId, &[&str])] = &[
        (rules::MODULE_IMPORT, &["+", ">", " math"]),
        (
            rules::MODULE_IMPORT,
            &["+> ", "alias ", ":", "=", " math/sin"],
        ),
        (rules::MODULE_IMPORT, &["+> math", "/", "_", "intrinsic"]),
        (
            rules::MODULE_IMPORT,
            &["+> math/", "{", "sin", ",", "cos", "}"],
        ),
        (rules::MODULE_IMPORT, &["+> math/{sin", "\n", "cos}"]),
        (rules::MODULE_IMPORT, &["+> math/{sin", "\t", "cos}"]),
        (
            rules::MODULE_IMPORT,
            &["+> ", "@", "ctx", " := ", "math/sin"],
        ),
        (rules::MODULE_IMPORT, &["+>", "\u{00a0}", "math"]),
        (rules::MODULE_IMPORT, &["+>", "\u{2009}", "math"]),
    ];

    for (rule, parts) in cases {
        let text = parts.concat();
        let contiguous = parse(source(&text), *rule, ParseConfig::default());
        let piece_backed = parse(piece_source(parts), *rule, ParseConfig::default());
        assert_snapshot_invariants(&contiguous, *rule, ParseConfig::default());
        assert_snapshot_invariants(&piece_backed, *rule, ParseConfig::default());
        assert_eq!(
            piece_backed.outcome, contiguous.outcome,
            "{rule:?} on {text:?}"
        );
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
            "{rule:?} on {text:?}",
        );
        assert_eq!(
            lowered_module_import(&piece_backed),
            lowered_module_import(&contiguous),
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
            larger_steps <= smaller_steps.saturating_mul(2).saturating_add(512),
            "parser steps were not linear: {measurements:?}",
        );
        assert!(
            larger_events <= smaller_events.saturating_mul(2).saturating_add(512),
            "parser events were not linear: {measurements:?}",
        );
    }
}

#[test]
fn closed_import_paths_and_groups_grow_linearly() {
    let sizes = [32_usize, 64, 128, 256];
    assert_linear(&measurements(
        rules::MODULE_IMPORT_PATH,
        sizes.into_iter().map(|size| vec!["a"; size].join("/")),
    ));
    assert_linear(&measurements(
        rules::IMPORT_GROUP_ITEMS,
        sizes.into_iter().map(|size| vec!["a"; size].join(",")),
    ));
}
