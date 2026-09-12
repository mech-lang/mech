use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, ParseLimits, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
    reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x2c7), Revision(0), text).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap()
}

fn node_with_text(node: &SyntaxNode, kind: SyntaxKind, text: &str) -> bool {
    (node.kind() == kind && node.text().unwrap() == text)
        || node
            .children()
            .any(|child| node_with_text(&child, kind, text))
}

fn recovered(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let parsed = parse(rule, text);
    assert_eq!(
        parsed.outcome,
        CanonicalRuleOutcome::Committed,
        "{text}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(
        parsed.consumed.end.0 as usize,
        text.len(),
        "{text}: {:?}",
        parsed.diagnostics
    );
    assert!(!parsed.diagnostics.is_empty(), "{text}");
    for diagnostic in parsed.diagnostics.iter() {
        assert!(diagnostic.rule.is_some(), "{text}");
        assert!(diagnostic.recovery.is_some(), "{text}");
    }
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        text
    );
    parsed
}

#[test]
fn recovered_generator_pattern_retains_arrow_source_and_later_qualifier() {
    for (rule, text) in [
        (rules::GENERATOR, "(x +) <- xs"),
        (rules::SET_COMPREHENSION, "{1 | (x +) <- xs}"),
        (rules::EXPRESSION, "{1 | (x +) <- xs, true}"),
        (rules::MATRIX_COMPREHENSION, "[1 | (x +) ← xs, true]"),
    ] {
        let parsed = recovered(rule, text);
        assert!(
            node_with_text(&parsed.syntax(), SyntaxKind::Variable, "xs"),
            "{text}"
        );
        assert!(
            node_with_text(
                &parsed.syntax(),
                SyntaxKind::Generator,
                if text.contains('←') {
                    "(x +) ← xs"
                } else {
                    "(x +) <- xs"
                }
            ),
            "{text}"
        );
        if text.contains("true") {
            assert!(
                node_with_text(&parsed.syntax(), SyntaxKind::ComprehensionQualifier, "true"),
                "{text}"
            );
        }
    }
}

#[test]
fn recovered_matrix_kind_retains_cardinality_suffix() {
    for (rule, text) in [
        (rules::KIND_MATRIX, "[<u8]:1,2"),
        (rules::KIND, "[<u8]:1,2"),
        (rules::KIND_ANNOTATION, "<[<u8]:1,2>"),
    ] {
        let parsed = recovered(rule, text);
        assert!(
            node_with_text(&parsed.syntax(), SyntaxKind::KindMatrix, "[<u8]:1,2"),
            "{text}"
        );
        for dimension in ["1", "2"] {
            assert!(
                node_with_text(&parsed.syntax(), SyntaxKind::Literal, dimension),
                "{text}: {dimension}"
            );
        }
    }
}

#[test]
fn recovered_variable_annotation_retains_definition_value() {
    for text in ["x<u8:> := 1", "~x<u8:> := 1", "x<u8:⟩ := (1, 2)"] {
        let parsed = recovered(rules::VARIABLE_DEFINE, text);
        assert!(
            node_with_text(&parsed.syntax(), SyntaxKind::VariableDefine, text),
            "{text}"
        );
        assert!(
            node_with_text(&parsed.syntax(), SyntaxKind::Literal, "1"),
            "{text}"
        );
    }
}

#[test]
fn missing_first_array_pattern_retains_later_item_and_physical_closer() {
    for (rule, text) in [
        (rules::PATTERN_ARRAY, "[, 1]"),
        (rules::PATTERN_ARRAY, "[, , 1]"),
        (rules::PATTERN, "[, 1]"),
        (rules::PATTERN_TUPLE, "([, 1], 2)"),
    ] {
        let parsed = recovered(rule, text);
        assert!(
            node_with_text(&parsed.syntax(), SyntaxKind::ArrayPatternElement, "1"),
            "{text}"
        );
        assert!(
            node_with_text(&parsed.syntax(), SyntaxKind::Missing, ""),
            "{text}"
        );
        let closers: Vec<_> = parsed
            .syntax()
            .tokens()
            .into_iter()
            .filter(|token| token.kind() == SyntaxKind::RightBracket)
            .collect();
        assert_eq!(closers.len(), 1, "{text}");
        assert_eq!(closers[0].text().unwrap(), "]", "{text}");
    }
}

#[test]
fn continuation_recovery_preserves_clean_grammar_selection() {
    for (rule, text) in [
        (rules::GENERATOR, "(x, y) <- xs"),
        (rules::COMPREHENSION_QUALIFIER, "x + 1"),
        (rules::SET_COMPREHENSION, "{1 | true}"),
        (rules::MATRIX_COMPREHENSION, "[x | x <- xs]"),
        (rules::KIND_MATRIX, "[u8]:1,2"),
        (rules::VARIABLE_DEFINE, "x<u8> := 1"),
        (rules::PATTERN_ARRAY, "[]"),
        (rules::PATTERN_ARRAY, "[1, 2]"),
        (rules::PATTERN_ARRAY, "[head, ..., tail]"),
    ] {
        let parsed = parse(rule, text);
        assert!(
            parsed.is_strictly_clean(),
            "{text}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.consumed.end.0 as usize, text.len(), "{text}");
    }
    for (rule, text) in [
        (rules::GENERATOR, "x"),
        (rules::GENERATOR, "x + 1"),
        (rules::VARIABLE_DEFINE, "x = 1"),
    ] {
        assert_eq!(
            parse(rule, text).outcome,
            CanonicalRuleOutcome::NoMatch,
            "{text}"
        );
    }
}

#[test]
fn recovered_continuations_obey_all_shared_resource_limits() {
    for (rule, text) in [
        (rules::GENERATOR, "(x +) <- xs"),
        (rules::EXPRESSION, "{1 | (x +) <- xs, true}"),
        (rules::KIND_MATRIX, "[<u8]:1,2"),
        (rules::VARIABLE_DEFINE, "x<u8:> := (1, 2)"),
        (rules::PATTERN_TUPLE, "([, , 1], 2)"),
    ] {
        let budgets = (0..=256)
            .map(|fuel| ParseLimits {
                fuel,
                ..ParseLimits::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=160).map(
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
            .chain((0..=4).map(|max_diagnostics| ParseLimits {
                max_diagnostics,
                ..ParseLimits::default()
            }))
            .chain((0..=16).map(|max_recovery_bytes| ParseLimits {
                max_recovery_bytes,
                ..ParseLimits::default()
            }));
        for limits in budgets {
            let parsed = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(
                    TextSnapshot::new(DocumentId(0x2c7), Revision(0), text).unwrap(),
                    rule,
                    ParseConfig { limits },
                )
            })
            .unwrap_or_else(|_| panic!("{text}: {limits:?}"))
            .unwrap();
            assert!(
                parsed.stats.parser_steps <= limits.fuel,
                "{text}: {limits:?}"
            );
            assert!(
                parsed.stats.events_emitted <= u64::from(limits.max_events),
                "{text}: {limits:?}"
            );
            assert!(
                parsed.stats.diagnostics_emitted <= u64::from(limits.max_diagnostics),
                "{text}: {limits:?}"
            );
            assert!(
                parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes),
                "{text}: {limits:?}"
            );
            validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
                .unwrap_or_else(|error| panic!("{text}: {limits:?}: {error:?}"));
            assert_eq!(
                reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
                &text[..parsed.consumed.end.0 as usize],
                "{limits:?}"
            );
        }
    }
}
