//! Kind child recovery must retain the selected owner's following syntax.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ExpectedSyntax, ParseConfig, ParseLimits, RecoveryAction, Revision, RuleId,
    SyntaxElement, SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
    reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(820), Revision(8), text).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap();
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        &text[..parsed.consumed.end.0 as usize]
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
fn only(node: &SyntaxNode, kind: SyntaxKind) -> SyntaxNode {
    let found = nodes(node, kind);
    assert_eq!(found.len(), 1, "{kind:?}: {node:?}");
    found[0].clone()
}
fn full(parsed: &CanonicalSourceRuleSnapshot) {
    assert_eq!(
        parsed.consumed,
        parsed.source.full_range(),
        "{:?}",
        parsed.diagnostics
    );
}
fn physical(node: &SyntaxNode, text: &str, offset: usize) {
    let token = node
        .children_with_tokens()
        .into_iter()
        .find_map(|element| match element {
            SyntaxElement::Token(token)
                if token.text().unwrap() == text && token.range().start.0 as usize == offset =>
            {
                Some(token)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing physical {text:?} in {:?}", node.kind()));
    assert!(
        !token
            .flags()
            .intersects(TokenFlags::ERROR | TokenFlags::MISSING | TokenFlags::SYNTHETIC)
    );
}
fn inserted_angle(parsed: &CanonicalSourceRuleSnapshot, rule: RuleId, at: usize) {
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(nodes(&parsed.syntax(), SyntaxKind::Error).is_empty());
    let diagnostic = parsed.diagnostics.iter().next().unwrap();
    let expected = ExpectedSyntax::Token(SyntaxKind::RightAngle);
    let range = TextRange::empty(TextSize(at as u32));
    assert_eq!(only(&parsed.syntax(), SyntaxKind::Missing).range(), range);
    assert_eq!(diagnostic.code.as_str(), "syntax/missing-delimiter");
    assert_eq!(diagnostic.rule, Some(rule));
    assert_eq!(
        diagnostic
            .primary
            .resolve(parsed.source.revision(), &parsed.nodes),
        Some(range)
    );
    assert_eq!(diagnostic.expected, vec![expected.clone()]);
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: expected,
            at: TextSize(at as u32)
        })
    );
}

#[test]
fn shared_kind_record_continues_after_a_recovered_first_annotation() {
    let text = "{ a<u8, b<u16>}";
    let parsed = parse(rules::KIND, text);
    full(&parsed);
    let record = only(&parsed.syntax(), SyntaxKind::KindRecord);
    assert_eq!(record.text().unwrap(), text);
    assert_eq!(
        nodes(&record, SyntaxKind::KindAnnotation)
            .iter()
            .map(|node| node.text().unwrap())
            .collect::<Vec<_>>(),
        ["<u8", "<u16>"]
    );
    assert!(nodes(&parsed.syntax(), SyntaxKind::KindMap).is_empty());
    assert!(nodes(&parsed.syntax(), SyntaxKind::KindSet).is_empty());
    physical(&record, "}", text.len() - 1);
    inserted_angle(&parsed, rules::KIND_ANNOTATION, text.find(',').unwrap());
}

#[test]
fn optional_kind_retains_question_after_a_recovered_matrix() {
    let text = "[<u8]?";
    let parsed = parse(rules::KIND_WITH_OPTION, text);
    full(&parsed);
    let option = nodes(&parsed.syntax(), SyntaxKind::KindWithOption)
        .into_iter()
        .find(|node| node.text().unwrap() == text)
        .unwrap();
    physical(&option, "?", 5);
    physical(&only(&option, SyntaxKind::KindMatrix), "]", 4);
    inserted_angle(&parsed, rules::KIND_KIND, 4);
}

#[test]
fn direct_recovered_matrix_keeps_its_valid_colon_only_suffix() {
    let text = "[<u8]:";
    let parsed = parse(rules::KIND_MATRIX, text);
    full(&parsed);
    let matrix = only(&parsed.syntax(), SyntaxKind::KindMatrix);
    physical(&matrix, ":", 5);
    physical(&matrix, "]", 4);
    inserted_angle(&parsed, rules::KIND_KIND, 4);
}

#[test]
fn enclosing_map_still_owns_colon_after_recovered_matrix_key() {
    for rule in [rules::KIND_MAP, rules::KIND, rules::KIND_ANNOTATION] {
        let text = if rule == rules::KIND_ANNOTATION {
            "<{[<u8]:u16}>"
        } else {
            "{[<u8]:u16}"
        };
        let parsed = parse(rule, text);
        full(&parsed);
        let map = only(&parsed.syntax(), SyntaxKind::KindMap);
        let kinds = map
            .children()
            .filter(|child| child.kind() == SyntaxKind::Kind)
            .collect::<Vec<_>>();
        assert_eq!(
            kinds
                .iter()
                .map(|kind| kind.text().unwrap())
                .collect::<Vec<_>>(),
            ["[<u8]", "u16"]
        );
        physical(&map, ":", text.find(':').unwrap());
        physical(&map, "}", text.find('}').unwrap());
        assert!(nodes(&parsed.syntax(), SyntaxKind::KindSet).is_empty());
        assert!(nodes(&parsed.syntax(), SyntaxKind::KindRecord).is_empty());
        inserted_angle(&parsed, rules::KIND_KIND, text.find(']').unwrap());
    }
}

#[test]
fn direct_and_shared_records_continue_first_and_later_recovered_fields() {
    for text in [
        "{a<u8, b<u16>}",
        "{ a<u8, b<u16>,…}",
        "{ a<u8>, b<u16, c<u32>}",
        "{ a⟨u8, b⟨u16⟩}",
    ] {
        for rule in [rules::KIND_RECORD, rules::KIND] {
            let parsed = parse(rule, text);
            full(&parsed);
            let record = only(&parsed.syntax(), SyntaxKind::KindRecord);
            let annotations = nodes(&record, SyntaxKind::KindAnnotation);
            assert_eq!(annotations.len(), if text.contains("c<") { 3 } else { 2 });
            assert!(
                annotations
                    .last()
                    .unwrap()
                    .text()
                    .unwrap()
                    .ends_with(['>', '⟩'])
            );
            assert!(nodes(&parsed.syntax(), SyntaxKind::KindMap).is_empty());
            assert!(nodes(&parsed.syntax(), SyntaxKind::KindSet).is_empty());
            physical(&record, "}", text.len() - 1);
            let at = if text.contains("u16,") {
                text.find("u16,").unwrap() + 3
            } else {
                text.find(',').unwrap()
            };
            inserted_angle(&parsed, rules::KIND_ANNOTATION, at);
        }
    }
}

#[test]
fn recovered_matrix_suffixes_compose_through_kind_and_annotation_owners() {
    for (rule, text, matrix_text) in [
        (rules::KIND, "[<u8]:", "[<u8]:"),
        (rules::KIND_ANNOTATION, "<[<u8]:>", "[<u8]:"),
        (rules::KIND_WITH_OPTION, "[<u8]:?", "[<u8]:"),
        (rules::KIND_ANNOTATION, "<[<u8]:?>", "[<u8]:"),
        (rules::KIND_WITH_OPTION, "[<u8]:1,2?", "[<u8]:1,2"),
        (rules::KIND_ANNOTATION, "<[<u8]:1,2?>", "[<u8]:1,2"),
        (rules::KIND_WITH_OPTION, "{ a<u8, b<u16>}?", ""),
    ] {
        let parsed = parse(rule, text);
        full(&parsed);
        if !matrix_text.is_empty() {
            let matrix = only(&parsed.syntax(), SyntaxKind::KindMatrix);
            assert_eq!(matrix.text().unwrap(), matrix_text);
            physical(&matrix, ":", text.find(':').unwrap());
            inserted_angle(&parsed, rules::KIND_KIND, text.find(']').unwrap());
        } else {
            inserted_angle(&parsed, rules::KIND_ANNOTATION, text.find(',').unwrap());
        }
        if let Some(at) = text.find('?') {
            let owner = nodes(&parsed.syntax(), SyntaxKind::KindWithOption)
                .into_iter()
                .find(|node| node.range().end.0 as usize == at + 1)
                .unwrap();
            physical(&owner, "?", at);
        }
    }
}

#[test]
fn matrix_colons_keep_clean_and_recovered_map_set_ownership() {
    for key in ["[u8]", "[u8]:1,2", "[u8]1,2", "[<u8]", "[<u8]:1,2"] {
        let text = format!("{{{key}:u16}}");
        for rule in [rules::KIND_MAP, rules::KIND] {
            let parsed = parse(rule, &text);
            full(&parsed);
            let map = only(&parsed.syntax(), SyntaxKind::KindMap);
            let children = map
                .children()
                .filter(|node| node.kind() == SyntaxKind::Kind)
                .collect::<Vec<_>>();
            assert_eq!(
                children
                    .iter()
                    .map(|node| node.text().unwrap())
                    .collect::<Vec<_>>(),
                [key, "u16"]
            );
            physical(&map, ":", 1 + key.len());
            physical(&map, "}", text.len() - 1);
            assert!(nodes(&parsed.syntax(), SyntaxKind::KindSet).is_empty());
            if key.contains('<') {
                inserted_angle(&parsed, rules::KIND_KIND, text.find(']').unwrap());
            } else {
                assert!(
                    parsed.is_strictly_clean(),
                    "{text}: {:?}",
                    parsed.diagnostics
                );
            }
        }
    }
    // A second colon can separate a bare matrix suffix from a matrix value.
    // An atom literal instead belongs to the matrix's greedy extent list.
    for key in ["[u8]:", "[<u8]:"] {
        let text = format!("{{{key}:[u16]}}");
        for rule in [rules::KIND_MAP, rules::KIND] {
            let parsed = parse(rule, &text);
            full(&parsed);
            let map = only(&parsed.syntax(), SyntaxKind::KindMap);
            let children = map
                .children()
                .filter(|node| node.kind() == SyntaxKind::Kind)
                .collect::<Vec<_>>();
            assert_eq!(
                children
                    .iter()
                    .map(|node| node.text().unwrap())
                    .collect::<Vec<_>>(),
                [key, "[u16]"]
            );
            physical(&map, ":", 1 + key.len());
            physical(&only(&children[0], SyntaxKind::KindMatrix), ":", key.len());
            if key.contains('<') {
                inserted_angle(&parsed, rules::KIND_KIND, text.find(']').unwrap());
            } else {
                assert!(parsed.is_strictly_clean());
            }
        }
    }
    let atom_extent = parse(rules::KIND, "{[u8]::u16}");
    full(&atom_extent);
    assert!(atom_extent.is_strictly_clean());
    assert_eq!(
        only(&atom_extent.syntax(), SyntaxKind::KindMatrix)
            .text()
            .unwrap(),
        "[u8]::u16"
    );
    assert!(nodes(&atom_extent.syntax(), SyntaxKind::KindMap).is_empty());
    for text in ["{[u8]:}", "{[u8]:}:2"] {
        for rule in [rules::KIND, rules::KIND_SET] {
            let parsed = parse(rule, text);
            full(&parsed);
            assert!(
                parsed.is_strictly_clean(),
                "{text}: {:?}",
                parsed.diagnostics
            );
            let set = only(&parsed.syntax(), SyntaxKind::KindSet);
            let matrix = only(&set, SyntaxKind::KindMatrix);
            assert_eq!(matrix.text().unwrap(), "[u8]:");
            physical(&matrix, ":", 5);
            assert!(nodes(&parsed.syntax(), SyntaxKind::KindMap).is_empty());
        }
    }
    let parsed = parse(rules::KIND_SET, "{[<u8]:}");
    full(&parsed);
    assert_eq!(
        only(&parsed.syntax(), SyntaxKind::KindMatrix)
            .text()
            .unwrap(),
        "[<u8]:"
    );
    inserted_angle(&parsed, rules::KIND_KIND, 5);
}

#[test]
fn clean_suffixes_and_nonmatching_rules_preserve_transactions() {
    for (rule, text) in [
        (rules::KIND_RECORD, "{ a<u8>, b<u16>}"),
        (rules::KIND, "{ a<u8>, b<u16>}"),
        (rules::KIND_MATRIX, "[u8]:"),
        (rules::KIND, "[u8]:"),
        (rules::KIND_WITH_OPTION, "[u8]:?"),
        (rules::KIND_ANNOTATION, "<[u8]:?>"),
    ] {
        let parsed = parse(rule, text);
        full(&parsed);
        assert!(
            parsed.is_strictly_clean(),
            "{text}: {:?}",
            parsed.diagnostics
        );
    }
    for (rule, text) in [
        (rules::KIND_WITH_OPTION, "?"),
        (rules::KIND_MATRIX, "?"),
        (rules::KIND_RECORD, "{x"),
    ] {
        let parsed = parse(rule, text);
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{text}");
        assert_eq!(parsed.consumed, TextRange::empty(TextSize(0)));
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.stats.recovery_bytes, 0);
    }
}

#[test]
fn kind_continuations_preserve_shared_resource_bounds() {
    for (rule, text) in [
        (rules::KIND, "{ a<u8, b<u16>}"),
        (rules::KIND_RECORD, "{a<u8, b<u16>}"),
        (rules::KIND_WITH_OPTION, "[<u8]:?"),
        (rules::KIND_MATRIX, "[<u8]:"),
        (rules::KIND, "{[<u8]:u16}"),
        (rules::KIND_MAP, "{[<u8]:@}"),
    ] {
        let limits = (0..=256)
            .map(|fuel| ParseLimits {
                fuel,
                ..ParseLimits::default()
            })
            .chain(
                (mech_syntax::document::parser::MIN_PREFIX_PRESERVING_EVENTS..=192).map(
                    |max_events| ParseLimits {
                        max_events,
                        ..ParseLimits::default()
                    },
                ),
            )
            .chain((0..=10).map(|max_nesting| ParseLimits {
                max_nesting,
                ..ParseLimits::default()
            }))
            .chain((0..=3).map(|max_diagnostics| ParseLimits {
                max_diagnostics,
                ..ParseLimits::default()
            }))
            .chain((0..=3).map(|max_recovery_bytes| ParseLimits {
                max_recovery_bytes,
                ..ParseLimits::default()
            }));
        for limits in limits {
            let parsed = std::panic::catch_unwind(|| {
                parse_canonical_phase_2i_rule_for_test(
                    TextSnapshot::new(DocumentId(820), Revision(8), text).unwrap(),
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
            validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
            assert_eq!(
                reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
                &text[..parsed.consumed.end.0 as usize]
            );
            for diagnostic in parsed.diagnostics.iter() {
                assert!(diagnostic.rule.is_some());
                assert!(
                    diagnostic
                        .primary
                        .resolve(parsed.source.revision(), &parsed.nodes)
                        .is_some()
                );
            }
        }
    }
}
