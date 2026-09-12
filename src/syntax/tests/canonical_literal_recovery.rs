use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2c_rule_for_test,
    parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ExpectedSyntax, FixApplicability, ParseConfig, ParseLimits, RecoveryAction,
    Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
    reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(921), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2c_rule_for_test(source(text), rule, ParseConfig::default()).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

#[test]
fn string_selection_rewinds_an_incomplete_raw_candidate_before_utf8_prefix() {
    let parsed = parse("\"\"\"abc", rules::STRING);
    assert!(parsed.is_strictly_clean());
    assert_eq!(parsed.consumed, TextRange::new(TextSize::ZERO, TextSize(2)));
    assert!(find_node(&parsed.syntax(), SyntaxKind::Utf8String).is_some());
    assert!(find_node(&parsed.syntax(), SyntaxKind::RawString).is_none());
    assert!(parsed.diagnostics.is_empty());

    let four_quotes = parse("\"\"\"\"", rules::STRING);
    assert!(four_quotes.is_strictly_clean());
    assert_eq!(four_quotes.consumed.end.0, 2);

    let raw = parse("\"\"\"\"\"\"", rules::STRING);
    assert!(raw.is_strictly_clean());
    assert_eq!(raw.consumed.end.0, 6);
    assert!(find_node(&raw.syntax(), SyntaxKind::RawString).is_some());
}

#[test]
fn direct_unclosed_strings_emit_only_their_own_structured_recovery() {
    let utf8 = parse("\"", rules::UTF8_STRING);
    assert!(utf8.matched);
    let diagnostic = utf8.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/unclosed-utf8-string");
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Token(SyntaxKind::Quote)]
    );
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: ExpectedSyntax::Token(SyntaxKind::Quote),
            at: TextSize(1),
        })
    );
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(
        diagnostic.fixes[0].applicability,
        FixApplicability::MachineApplicable
    );

    let raw = parse("\"\"\"unterminated", rules::RAW_STRING);
    assert!(raw.matched);
    let diagnostic = raw.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/unclosed-raw-string");
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Production("triple closing quote".into())]
    );
    let missing = find_node(&raw.syntax(), SyntaxKind::Missing).unwrap();
    let missing_quotes = missing
        .tokens()
        .into_iter()
        .filter(|token| token.kind() == SyntaxKind::Quote)
        .collect::<Vec<_>>();
    assert_eq!(missing_quotes.len(), 3);
    assert!(missing_quotes.iter().all(|token| {
        token.range() == TextRange::empty(TextSize(15))
            && token.flags().contains(TokenFlags::MISSING)
    }));
}

#[test]
fn based_prefixes_commit_missing_payload_recovery_with_physical_found_syntax() {
    for (rule, input, code, expected, at) in [
        (
            rules::DECIMAL_LITERAL,
            "0d",
            "syntax/missing-decimal-digits",
            "decimal digits",
            2,
        ),
        (
            rules::DECIMAL_LITERAL,
            "0dX",
            "syntax/missing-decimal-digits",
            "decimal digits",
            2,
        ),
        (
            rules::HEXADECIMAL_LITERAL,
            "0x",
            "syntax/missing-hexadecimal-digits",
            "hexadecimal digits",
            2,
        ),
        (
            rules::OCTAL_LITERAL,
            "0o",
            "syntax/missing-octal-digits",
            "octal digits",
            2,
        ),
        (
            rules::BINARY_LITERAL,
            "0b",
            "syntax/missing-binary-digits",
            "binary digits",
            2,
        ),
    ] {
        let parsed = parse(input, rule);
        assert!(parsed.matched, "{input:?}");
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code.as_str(), code, "{input:?}");
        assert_eq!(
            diagnostic.expected,
            vec![ExpectedSyntax::Production(expected.into())],
            "{input:?}",
        );
        assert_eq!(
            diagnostic.recovery,
            Some(RecoveryAction::Insert {
                syntax: ExpectedSyntax::Production(expected.into()),
                at: TextSize(at),
            }),
            "{input:?}",
        );
        assert!(diagnostic.fixes.is_empty(), "{input:?}");
    }
}

#[test]
fn missing_hexadecimal_payload_is_a_production_without_a_synthetic_digit() {
    let parsed = parse("0x", rules::HEXADECIMAL_LITERAL);
    let missing = find_node(&parsed.syntax(), SyntaxKind::Missing)
        .expect("the missing hexadecimal payload has a MISSING node");

    assert!(
        missing.children_with_tokens().is_empty(),
        "a hexadecimal payload is a production, so recovery must not invent a digit token",
    );
    assert!(
        missing.tokens().is_empty(),
        "the MISSING node must contain no synthetic tokens",
    );
}

#[test]
fn incomplete_losing_candidates_restore_without_diagnostics() {
    for (rule, input) in [
        (rules::ATOM, ":"),
        (rules::RATIONAL_LITERAL, "1/"),
        (rules::FLOAT_FULL, "1."),
        (rules::COMPLEX_NUMBER, "1+"),
    ] {
        let parsed = parse(input, rule);
        assert!(!parsed.matched, "{rule:?} on {input:?}");
        assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{input:?}"
        );
    }
}

#[test]
fn fuel_exhausted_at_string_recovery_opening_retains_its_selected_owner() {
    for (rule, text, fuel, parent) in [
        (
            rules::STRING,
            "\"unterminated",
            14,
            SyntaxKind::StringLiteral,
        ),
        (
            rules::EXPRESSION,
            "\"unterminated",
            14,
            SyntaxKind::Expression,
        ),
        (
            rules::VARIABLE_DEFINE,
            "x := (|a<*>|1 |b<*>|\"unterminated| |)",
            93,
            SyntaxKind::VariableDefine,
        ),
        (
            rules::VARIABLE_DEFINE,
            "x := (|a<*>|1 |b<*>|\"unterminated| |)",
            141,
            SyntaxKind::VariableDefine,
        ),
    ] {
        let config = ParseConfig {
            limits: ParseLimits {
                fuel,
                ..ParseLimits::default()
            },
        };
        let parsed = parse_canonical_phase_2c_rule_for_test(source(text), rule, config)
            .or_else(|| parse_canonical_phase_2i_rule_for_test(source(text), rule, config))
            .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(parsed.stats.parser_steps, fuel);
        assert_eq!(parsed.consumed, parsed.source.full_range());
        assert!(find_node(&parsed.syntax(), parent).is_some());
        let string = find_node(&parsed.syntax(), SyntaxKind::StringLiteral).unwrap();
        let utf8 = find_node(&string, SyntaxKind::Utf8String).unwrap();
        let remainder = find_node(&utf8, SyntaxKind::Error).unwrap();
        assert_eq!(remainder.range().start.0 as usize, text.find('"').unwrap());
        assert_eq!(remainder.range().end, parsed.consumed.end);
        let resource_diagnostics = parsed
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit")
            .collect::<Vec<_>>();
        assert_eq!(resource_diagnostics.len(), 1);
        if rule != rules::VARIABLE_DEFINE {
            assert_eq!(parsed.diagnostics.len(), 1);
        }
        let diagnostic = resource_diagnostics[0];
        assert_eq!(diagnostic.code.as_str(), "syntax/recovery-limit");
        assert_eq!(diagnostic.rule, Some(rules::STRING));
        assert_eq!(diagnostic.context, None);
        assert!(diagnostic.expected.is_empty());
        assert_eq!(
            diagnostic.found.as_ref().unwrap().kind,
            Some(SyntaxKind::Quote)
        );
        assert_eq!(
            diagnostic.found.as_ref().unwrap().text.as_deref(),
            Some("\"")
        );
        assert_eq!(
            diagnostic.recovery,
            Some(RecoveryAction::ResourceLimit {
                range: remainder.range(),
            })
        );
        assert!(diagnostic.fixes.is_empty());
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            text
        );
    }
}

#[test]
fn direct_string_recovery_preserves_its_owner_when_opening_quotes_exhaust_fuel() {
    for (rule, text, kind, fuel, physical_quotes) in [
        (
            rules::UTF8_STRING,
            "\"unterminated",
            SyntaxKind::Utf8String,
            13,
            0,
        ),
        (
            rules::RAW_STRING,
            "\"\"\"unterminated",
            SyntaxKind::RawString,
            15,
            0,
        ),
        (
            rules::RAW_STRING,
            "\"\"\"unterminated",
            SyntaxKind::RawString,
            16,
            1,
        ),
        (
            rules::RAW_STRING,
            "\"\"\"unterminated",
            SyntaxKind::RawString,
            17,
            2,
        ),
    ] {
        let parsed = parse_canonical_phase_2c_rule_for_test(
            source(text),
            rule,
            ParseConfig {
                limits: ParseLimits {
                    fuel,
                    ..ParseLimits::default()
                },
            },
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(parsed.stats.parser_steps, fuel);
        assert_eq!(parsed.consumed, parsed.source.full_range());
        let owner = find_node(&parsed.syntax(), kind).unwrap();
        assert_eq!(owner.range(), parsed.consumed);
        let remainder = find_node(&owner, SyntaxKind::Error).unwrap();
        assert_eq!(
            remainder.range(),
            TextRange::new(TextSize(physical_quotes), parsed.consumed.end)
        );
        assert_eq!(
            owner
                .tokens()
                .iter()
                .filter(|token| token.kind() == SyntaxKind::Quote)
                .count(),
            physical_quotes as usize
        );
        assert_eq!(parsed.diagnostics.len(), 1);
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code.as_str(), "syntax/recovery-limit");
        assert_eq!(diagnostic.rule, Some(rule));
        assert_eq!(diagnostic.context, None);
        assert!(diagnostic.expected.is_empty());
        assert_eq!(
            diagnostic.found.as_ref().unwrap().kind,
            Some(SyntaxKind::Quote)
        );
        assert_eq!(
            diagnostic.found.as_ref().unwrap().text.as_deref(),
            Some("\"")
        );
        assert_eq!(
            diagnostic.recovery,
            Some(RecoveryAction::ResourceLimit {
                range: remainder.range()
            })
        );
        assert!(diagnostic.fixes.is_empty());
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            text
        );
    }
}

#[test]
fn string_recovery_keeps_all_shared_resource_budgets_bounded() {
    for (rule, text) in [
        (rules::STRING, "\"unterminated"),
        (rules::UTF8_STRING, "\"unterminated"),
        (rules::RAW_STRING, "\"\"\"unterminated"),
        (rules::STRING, "\"closed\""),
        (rules::STRING, "\"\"\"closed\"\"\""),
        (rules::EXPRESSION, "\"unterminated"),
        (
            rules::VARIABLE_DEFINE,
            "x := (|a<*>|1 |b<*>|\"unterminated| |)",
        ),
    ] {
        for limits in (0..=180)
            .map(|fuel| ParseLimits {
                fuel,
                ..ParseLimits::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=120).map(
                    |max_events| ParseLimits {
                        max_events,
                        ..ParseLimits::default()
                    },
                ),
            )
            .chain((0..=8).map(|max_nesting| ParseLimits {
                max_nesting,
                ..ParseLimits::default()
            }))
            .chain((0..=2).flat_map(|max_diagnostics| {
                (0..=24).map(move |max_recovery_bytes| ParseLimits {
                    max_diagnostics,
                    max_recovery_bytes,
                    ..ParseLimits::default()
                })
            }))
        {
            let config = ParseConfig { limits };
            let parsed = std::panic::catch_unwind(|| {
                parse_canonical_phase_2c_rule_for_test(source(text), rule, config)
                    .or_else(|| parse_canonical_phase_2i_rule_for_test(source(text), rule, config))
                    .unwrap()
            })
            .unwrap_or_else(|_| panic!("{rule:?}, {text:?}, {limits:?}"));
            assert!(parsed.stats.parser_steps <= limits.fuel, "{limits:?}");
            assert!(
                parsed.stats.events_emitted <= u64::from(limits.max_events),
                "{limits:?}"
            );
            assert!(
                parsed.stats.diagnostics_emitted <= u64::from(limits.max_diagnostics),
                "{limits:?}"
            );
            assert!(
                parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes),
                "{limits:?}"
            );
            validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
                .unwrap_or_else(|error| panic!("{rule:?}, {text:?}, {limits:?}: {error:?}"));
            assert_eq!(
                reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
                &text[..parsed.consumed.end.0 as usize],
                "{rule:?}, {limits:?}"
            );
        }
    }
}
