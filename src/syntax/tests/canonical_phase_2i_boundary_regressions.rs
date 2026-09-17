//! Regression evidence for recovery boundaries found during S2 qualification.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DiagnosticPhase, DocumentId, ParseConfig, RecoveryAction, Revision, RuleId, Severity,
    SyntaxElement, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
    reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(0x522), Revision(8), text).unwrap();
    let parsed =
        parse_canonical_phase_2i_rule_for_test(source, rule, ParseConfig::default()).unwrap();
    assert_eq!(
        parsed.outcome,
        CanonicalRuleOutcome::Committed,
        "{rule:?}: {text:?}"
    );
    assert_eq!(
        parsed.consumed,
        TextRange::new(TextSize::ZERO, TextSize(text.len() as u32)),
        "{rule:?}: {text:?}"
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        text
    );
    parsed
}
fn nodes(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut found = Vec::new();
    if node.kind() == kind {
        found.push(node.clone());
    }
    for child in node.children() {
        found.extend(nodes(&child, kind));
    }
    found
}
fn owner(node: &SyntaxNode, kind: SyntaxKind) -> SyntaxNode {
    nodes(node, kind)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("missing {kind:?}"))
}
fn physical(node: &SyntaxNode, text: &str, at: usize) {
    let token = node
        .children_with_tokens()
        .into_iter()
        .find_map(|item| match item {
            SyntaxElement::Token(token)
                if token.range().start == TextSize(at as u32) && token.text().unwrap() == text =>
            {
                Some(token)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing physical {text:?} at {at} inside {:?}", node.kind()));
    assert!(
        !token
            .flags()
            .intersects(TokenFlags::MISSING | TokenFlags::SYNTHETIC | TokenFlags::ERROR)
    );
}
fn diagnostic(
    parsed: &CanonicalSourceRuleSnapshot,
    code: &str,
    rule: RuleId,
    at: usize,
    missing: bool,
) {
    let found = parsed
        .diagnostics
        .iter()
        .filter(|item| item.code.as_str() == code)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "{code}: {:?}", parsed.diagnostics);
    let found = found[0];
    assert_eq!(found.phase, DiagnosticPhase::Syntax);
    assert_eq!(found.severity, Severity::Error);
    assert_eq!(found.rule, Some(rule));
    assert!(
        found
            .primary
            .resolve(parsed.source.revision(), &parsed.nodes)
            .is_some()
    );
    if missing {
        assert!(
            matches!(found.recovery,Some(RecoveryAction::Insert{at:actual,..}) if actual==TextSize(at as u32))
        );
        assert_eq!(
            found
                .primary
                .resolve(parsed.source.revision(), &parsed.nodes),
            Some(TextRange::empty(TextSize(at as u32)))
        );
    } else {
        assert_eq!(
            found.recovery,
            Some(RecoveryAction::Abandon {
                rule,
                at: TextSize(at as u32)
            })
        );
    }
}

// 3996556025
#[test]
fn recovery_balances_ascii_and_unicode_angles_before_argument_restart() {
    for annotation in ["<u8>", "⟨u8⟩", "<⟨u8⟩>", "<u8⟩", "⟨u8>"] {
        let arguments = format!("(1 + @ {annotation}, 3)");
        for (rule, source) in [
            (rules::ARGUMENT_LIST, arguments.clone()),
            (rules::EXPRESSION, format!("f{arguments}")),
        ] {
            let parsed = parse(rule, &source);
            let list = owner(&parsed.syntax(), SyntaxKind::ArgumentList);
            assert_eq!(nodes(&list, SyntaxKind::CallArgument).len(), 2);
            assert_eq!(
                nodes(&list, SyntaxKind::IntegerLiteral)
                    .iter()
                    .map(|node| node.text().unwrap())
                    .collect::<Vec<_>>(),
                ["1", "3"]
            );
            assert_eq!(
                nodes(&list, SyntaxKind::Error)
                    .iter()
                    .map(|node| node.text().unwrap())
                    .collect::<Vec<_>>(),
                [format!("@ {annotation}")]
            );
            assert!(nodes(&list, SyntaxKind::Missing).is_empty());
            physical(&list, ")", source.len() - 1);
            diagnostic(
                &parsed,
                "syntax/unexpected-production-source",
                rules::L3,
                source.find(',').unwrap(),
                false,
            );
        }
    }
}

// 3996556046. Unicode arrows may themselves be valid emoji variables; the
// rejected wildcard makes the missing-value recovery path unambiguous.
#[test]
fn missing_fsm_values_preserve_unicode_later_stages() {
    for (source, kind, code, rule, missing) in [
        (
            "# ⇒ :value",
            SyntaxKind::FsmOutput,
            "syntax/missing-fsm-name",
            rules::FSM_INSTANCE,
            true,
        ),
        (
            "# → :next",
            SyntaxKind::FsmStateTransition,
            "syntax/missing-fsm-name",
            rules::FSM_INSTANCE,
            true,
        ),
        (
            "#m -> * → :next",
            SyntaxKind::FsmStateTransition,
            "syntax/unexpected-production-source",
            rules::FSM_STATE_TRANSITION,
            false,
        ),
        (
            "#m -> * ⇒ :value",
            SyntaxKind::FsmOutput,
            "syntax/unexpected-production-source",
            rules::FSM_STATE_TRANSITION,
            false,
        ),
    ] {
        for root in [rules::FSM_PIPE, rules::EXPRESSION] {
            let parsed = parse(root, source);
            let pipe = owner(&parsed.syntax(), SyntaxKind::FsmPipe);
            assert_eq!(pipe.text().unwrap(), source);
            let stages = nodes(&pipe, kind);
            let stage = stages.last().unwrap();
            assert_eq!(
                stage.text().unwrap(),
                if kind == SyntaxKind::FsmOutput {
                    "⇒ :value"
                } else {
                    "→ :next"
                }
            );
            assert_eq!(
                nodes(&pipe, SyntaxKind::Missing).len(),
                usize::from(missing)
            );
            assert_eq!(nodes(&pipe, SyntaxKind::Error).len(), usize::from(!missing));
            let glyph = if kind == SyntaxKind::FsmOutput {
                '⇒'
            } else {
                '→'
            };
            let at = source.find(glyph).unwrap();
            physical(stage, &glyph.to_string(), at);
            diagnostic(&parsed, code, rule, at, missing);
        }
    }
}

// 3996556049. The canonical emoji exclusion includes ╯ but permits ┘/┛ as
// identifiers. An invalid @ prefix exercises recovery before those closers
// without changing their legal expression role.
#[test]
fn missing_operands_preserve_all_physical_framed_matrix_closers() {
    for source in ["╭1 +╯", "╭1 + @ ╯", "┌1 + @ ┘", "┏1 + @ ┛"] {
        let closing = source.chars().last().unwrap();
        let missing = !source.contains('@');
        for rule in [rules::MATRIX, rules::EXPRESSION] {
            let parsed = parse(rule, source);
            let matrix = owner(&parsed.syntax(), SyntaxKind::Matrix);
            assert_eq!(matrix.text().unwrap(), source);
            assert_eq!(nodes(&matrix, SyntaxKind::IntegerLiteral).len(), 1);
            assert_eq!(
                nodes(&matrix, SyntaxKind::Missing).len(),
                usize::from(missing)
            );
            assert_eq!(
                nodes(&matrix, SyntaxKind::Error).len(),
                usize::from(!missing)
            );
            let at = source.len() - closing.len_utf8();
            physical(&matrix, &closing.to_string(), at);
            diagnostic(
                &parsed,
                if missing {
                    "syntax/missing-operator-operand"
                } else {
                    "syntax/unexpected-production-source"
                },
                rules::L3,
                at,
                missing,
            );
        }
    }
}

// 3996556058
#[test]
fn absent_match_patterns_preserve_outputs_and_later_arms() {
    for output in ["=>"] {
        for (rule, source, values) in [
            (rules::MATCH_ARM, format!("| {output} 2"), vec!["2"]),
            (
                rules::EXPRESSION,
                format!("x ? | {output} 2 | * {output} 3"),
                vec!["2", "3"],
            ),
        ] {
            let parsed = parse(rule, &source);
            let arms = nodes(&parsed.syntax(), SyntaxKind::MatchArm);
            assert_eq!(arms.len(), values.len());
            assert_eq!(
                arms.iter()
                    .flat_map(|arm| nodes(arm, SyntaxKind::IntegerLiteral))
                    .map(|node| node.text().unwrap())
                    .collect::<Vec<_>>(),
                values
            );
            assert_eq!(nodes(&arms[0], SyntaxKind::Missing).len(), 1);
            assert!(nodes(&parsed.syntax(), SyntaxKind::Error).is_empty());
            physical(&arms[0], output, source.find(output).unwrap());
            diagnostic(
                &parsed,
                "syntax/missing-match-arm-pattern",
                rules::MATCH_ARM,
                source.find(output).unwrap(),
                true,
            );
        }
    }
}

#[test]
fn invalid_match_pattern_stops_at_unicode_output_without_reserving_emoji() {
    for (rule, source) in [
        (rules::MATCH_ARM, "| @ ⇒ 2"),
        (rules::EXPRESSION, "x ? | @ ⇒ 2 | * => 3"),
    ] {
        let parsed = parse(rule, source);
        let arm = owner(&parsed.syntax(), SyntaxKind::MatchArm);
        assert_eq!(nodes(&arm, SyntaxKind::Error).len(), 1);
        assert!(nodes(&arm, SyntaxKind::Missing).is_empty());
        assert_eq!(
            nodes(&arm, SyntaxKind::IntegerLiteral)[0].text().unwrap(),
            "2"
        );
        physical(&arm, "⇒", source.find('⇒').unwrap());
        diagnostic(
            &parsed,
            "syntax/unexpected-production-source",
            rules::MATCH_ARM,
            source.find('⇒').unwrap(),
            false,
        );
    }
}

#[test]
fn recovery_boundaries_do_not_reserve_valid_emoji_identifiers() {
    for glyph in ["→", "⇒", "┘", "┛"] {
        let source = TextSnapshot::new(DocumentId(0x522), Revision(8), glyph).unwrap();
        let parsed =
            parse_canonical_phase_2i_rule_for_test(source, rules::FACTOR, ParseConfig::default())
                .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched);
        assert!(parsed.is_strictly_clean());
        assert_eq!(parsed.consumed.end, TextSize(glyph.len() as u32));
        assert_eq!(
            owner(&parsed.syntax(), SyntaxKind::Identifier)
                .text()
                .unwrap(),
            glyph
        );
    }
}

#[test]
fn recovery_distinguishes_operator_prefixes_from_angle_and_output_boundaries() {
    for source in ["(1 + @ {x | x <- xs}, 3)", "(1 + @ (x <= y), 3)"] {
        let parsed = parse(rules::ARGUMENT_LIST, source);
        let list = owner(&parsed.syntax(), SyntaxKind::ArgumentList);
        assert_eq!(nodes(&list, SyntaxKind::CallArgument).len(), 2);
        assert_eq!(
            nodes(&list, SyntaxKind::IntegerLiteral)
                .last()
                .unwrap()
                .text()
                .unwrap(),
            "3"
        );
        physical(&list, ")", source.len() - 1);
    }
    let source = "| @ = nope => 2";
    let parsed = parse(rules::MATCH_ARM, source);
    let arm = owner(&parsed.syntax(), SyntaxKind::MatchArm);
    assert_eq!(
        nodes(&arm, SyntaxKind::Error)[0].text().unwrap(),
        "@ = nope "
    );
    assert_eq!(
        nodes(&arm, SyntaxKind::IntegerLiteral)[0].text().unwrap(),
        "2"
    );
    physical(&arm, "=>", source.find("=>").unwrap());
    diagnostic(
        &parsed,
        "syntax/unexpected-production-source",
        rules::MATCH_ARM,
        source.find("=>").unwrap(),
        false,
    );
    for kind in ["1<u8⟩", "1⟨u8>"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(0x522), Revision(8), kind).unwrap(),
            rules::LITERAL,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean());
        assert_eq!(parsed.consumed.end, TextSize(kind.len() as u32));
    }
}
