//! Recovery retains each structure's suffix and later children.
use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2i_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot, TokenFlags,
    reconstruct_source_range, validate_lossless_range,
};

fn parse(rule: RuleId, text: &str) -> CanonicalSourceRuleSnapshot {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(820), Revision(4), text).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(
        parsed.outcome,
        CanonicalRuleOutcome::Committed,
        "{rule:?}: {text}"
    );
    assert_eq!(
        parsed.consumed,
        parsed.source.full_range(),
        "{rule:?}: {text}"
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        text
    );
    assert_eq!(
        parsed.diagnostics.len(),
        1,
        "{rule:?}: {text}: {:?}",
        parsed.diagnostics
    );
    for diagnostic in parsed.diagnostics.iter() {
        assert!(diagnostic.rule.is_some());
        assert!(
            diagnostic
                .primary
                .resolve(parsed.source.revision(), &parsed.nodes)
                .is_some()
        );
        assert!(diagnostic.recovery.is_some());
    }
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
fn physical(node: &SyntaxNode, text: &str, at: usize) {
    let tokens = node.tokens();
    let token = tokens
        .iter()
        .find(|token| token.text().unwrap() == text && token.range().start.0 as usize == at)
        .unwrap_or_else(|| panic!("missing {text:?} at {at} in {:?}", node.kind()));
    assert!(
        !token
            .flags()
            .intersects(TokenFlags::ERROR | TokenFlags::MISSING | TokenFlags::SYNTHETIC)
    );
}

#[test]
fn recovered_mapping_value_owns_comma_and_trailing_whitespace() {
    for (rule, text) in [
        (rules::MAPPING, "1: (2 +), "),
        (rules::MAP, "{1: (2 +), 3: 4}"),
        (rules::EXPRESSION, "{1: (2 +), 3: 4}"),
    ] {
        let parsed = parse(rule, text);
        let entries = nodes(&parsed.syntax(), SyntaxKind::MapEntry);
        assert_eq!(entries[0].text().unwrap(), "1: (2 +), ");
        physical(&entries[0], ",", text.find(',').unwrap());
        assert_eq!(entries.len(), if rule == rules::MAPPING { 1 } else { 2 });
    }
}
#[test]
fn missing_binding_value_owns_comma_and_trailing_whitespace() {
    for (rule, text) in [
        (rules::BINDING, "a:, "),
        (rules::RECORD, "{a:, b: 2}"),
        (rules::EXPRESSION, "{a:, b: 2}"),
    ] {
        let parsed = parse(rule, text);
        let bindings = nodes(&parsed.syntax(), SyntaxKind::RecordBinding);
        assert_eq!(bindings[0].text().unwrap(), "a:, ");
        physical(&bindings[0], ",", text.find(',').unwrap());
        assert_eq!(bindings.len(), if rule == rules::BINDING { 1 } else { 2 });
    }
}
#[test]
fn inline_row_retains_cell_after_recovered_expression() {
    let text = "1 (2 +) 3|";
    let parsed = parse(rules::INLINE_TABLE_ROW, text);
    let row = &nodes(&parsed.syntax(), SyntaxKind::InlineTableRow)[0];
    let cells = row
        .children()
        .filter(|node| node.kind() == SyntaxKind::Expression)
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[2].text().unwrap().trim(), "3");
    physical(row, "|", text.len() - 1);
    assert!(nodes(row, SyntaxKind::Error).is_empty());
}
#[test]
fn headers_retain_field_after_recovered_annotation() {
    for (rule, kind) in [
        (rules::TABLE_HEADER, SyntaxKind::TableHeader),
        (rules::INLINE_TABLE_HEADER, SyntaxKind::InlineTableHeader),
    ] {
        let text = "a<u8> b<[u16> c<u32>|";
        let parsed = parse(rule, text);
        let header = &nodes(&parsed.syntax(), kind)[0];
        let fields = nodes(header, SyntaxKind::HeaderField);
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[2].text().unwrap(), "c<u32>");
        physical(header, "|", text.len() - 1);
        assert!(nodes(header, SyntaxKind::Error).is_empty());
    }
}
#[test]
fn regular_row_retains_cell_after_recovered_expression() {
    let text = "|1 (2 +) 3|";
    let parsed = parse(rules::TABLE_ROW, text);
    let row = &nodes(&parsed.syntax(), SyntaxKind::TableRow)[0];
    let cells = row
        .children()
        .filter(|node| node.kind() == SyntaxKind::Expression)
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[2].text().unwrap().trim(), "3");
    physical(row, "|", text.len() - 1);
    assert!(nodes(row, SyntaxKind::Error).is_empty());
}
#[test]
fn framed_row_resumes_after_missing_first_cell() {
    let text = "││1│";
    let parsed = parse(rules::TABLE_ROW2, text);
    let row = &nodes(&parsed.syntax(), SyntaxKind::FancyTableRow)[0];
    let cells = row
        .children()
        .filter(|node| node.kind() == SyntaxKind::Expression)
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].text().unwrap(), "1");
    physical(row, "│", 3);
    physical(row, "│", 7);
    assert_eq!(nodes(row, SyntaxKind::Missing).len(), 1);
    assert!(nodes(row, SyntaxKind::Error).is_empty());
}

#[test]
fn suffix_recovery_preserves_later_entries_and_shared_table_owners() {
    for (rule, text, owner, child, count) in [
        (
            rules::MAP,
            "{1: 2, 3:, 5: 6}",
            SyntaxKind::Map,
            SyntaxKind::MapEntry,
            3,
        ),
        (
            rules::EXPRESSION,
            "{1: 2, 3:, 5: 6}",
            SyntaxKind::Map,
            SyntaxKind::MapEntry,
            3,
        ),
        (
            rules::EXPRESSION,
            "{a: 1, b: (2 +), c: 3}",
            SyntaxKind::Record,
            SyntaxKind::RecordBinding,
            3,
        ),
        (
            rules::RECORD,
            "{a: 1, b: (2 +), c: 3}",
            SyntaxKind::Record,
            SyntaxKind::RecordBinding,
            3,
        ),
        (
            rules::INLINE_TABLE,
            "|a<u8> b<u8> c<u8>|1 (2 +) 3|",
            SyntaxKind::InlineTableRow,
            SyntaxKind::Expression,
            3,
        ),
        (
            rules::EXPRESSION,
            "|a<u8> b<u8> c<u8>|1 (2 +) 3|",
            SyntaxKind::InlineTableRow,
            SyntaxKind::Expression,
            3,
        ),
        (
            rules::TABLE_ROW,
            "|(1 +) 2 3|",
            SyntaxKind::TableRow,
            SyntaxKind::Expression,
            3,
        ),
        (
            rules::INLINE_TABLE_ROW,
            "(1 +) 2 3|",
            SyntaxKind::InlineTableRow,
            SyntaxKind::Expression,
            3,
        ),
        (
            rules::TABLE_HEADER,
            "a<[u8> b<u16> c<u32>|",
            SyntaxKind::TableHeader,
            SyntaxKind::HeaderField,
            3,
        ),
        (
            rules::INLINE_TABLE_HEADER,
            "a<[u8> b<u16> c<u32>|",
            SyntaxKind::InlineTableHeader,
            SyntaxKind::HeaderField,
            3,
        ),
    ] {
        let parsed = parse(rule, text);
        let owners = nodes(&parsed.syntax(), owner);
        assert_eq!(owners.len(), 1, "{rule:?}: {text}");
        assert_eq!(
            owners[0]
                .children()
                .filter(|node| node.kind() == child)
                .count(),
            count,
            "{rule:?}: {text}"
        );
        assert!(nodes(&owners[0], SyntaxKind::Error).is_empty(), "{text}");
        if owner == SyntaxKind::Map || owner == SyntaxKind::Record {
            physical(&owners[0], "}", text.len() - 1);
            for entry in owners[0]
                .children()
                .filter(|node| node.kind() == child)
                .take(2)
            {
                assert!(
                    entry.text().unwrap().ends_with(", "),
                    "{text}: {:?}",
                    entry.text()
                );
            }
        }
    }
    for text in ["││1│", "┃┃1┃", "│ │1│"] {
        let parsed = parse(rules::TABLE_ROW2, text);
        assert_eq!(nodes(&parsed.syntax(), SyntaxKind::FancyTableRow).len(), 1);
        assert_eq!(parsed.diagnostics.len(), 1);
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code.as_str(), "syntax/missing-table-cell");
        assert_eq!(diagnostic.rule, Some(rules::TABLE_ROW2));
    }
}

#[test]
fn structure_suffixes_preserve_clean_inputs_and_ancestor_boundaries() {
    for (rule, text) in [
        (rules::MAPPING, "1: 2, "),
        (rules::BINDING, "a: 1, "),
        (rules::MAP, "{1: 2, 3: 4}"),
        (rules::RECORD, "{a: 1, b: 2}"),
        (rules::INLINE_TABLE_ROW, "1 2 3|"),
        (rules::TABLE_ROW, "|1 2 3|"),
        (rules::TABLE_ROW2, "│1│2│"),
        (rules::TABLE_HEADER, "a<u8> b<u16> c<u32>|"),
        (rules::INLINE_TABLE_HEADER, "a<u8> b<u16> c<u32>|"),
    ] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(820), Revision(4), text).unwrap(),
            rule,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{rule:?}: {text}");
        assert_eq!(parsed.consumed, parsed.source.full_range());
    }
    for (rule, text) in [
        (rules::MAPPING, "1: (2 +), }"),
        (rules::BINDING, "a:, }"),
        (rules::TABLE_ROW2, "││1│}"),
    ] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(820), Revision(4), text).unwrap(),
            rule,
            ParseConfig::default(),
        )
        .unwrap();
        assert_eq!(parsed.outcome, CanonicalRuleOutcome::Committed);
        assert_eq!(parsed.consumed.end.0 as usize, text.len() - 1);
        validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    }
}

#[test]
fn suffix_continuations_obey_resource_limits_and_piece_source_ownership() {
    use mech_syntax::document::ParseLimits;
    for (rule, text) in [
        (rules::MAPPING, "1: (2 +), "),
        (rules::BINDING, "a:, "),
        (rules::EXPRESSION, "{a: 1, b: (2 +), c: 3}"),
        (rules::EXPRESSION, "{1: 2, 3:, 5: 6}"),
        (rules::INLINE_TABLE_ROW, "1 (2 +) 3|"),
        (rules::TABLE_HEADER, "a<u8> b<[u16> c<u32>|"),
        (rules::INLINE_TABLE_HEADER, "a<u8> b<[u16> c<u32>|"),
        (rules::TABLE_ROW, "|1 (2 +) 3|"),
        (rules::TABLE_ROW2, "││1│"),
    ] {
        let source = text.chars().fold(
            TextSnapshot::new(DocumentId(820), Revision(4), "").unwrap(),
            |source, character| source.append(character.to_string()).unwrap(),
        );
        assert!(source.piece_count() > 1);
        let limits = (0..=180)
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
                (0..=16).map(move |max_recovery_bytes| ParseLimits {
                    max_diagnostics,
                    max_recovery_bytes,
                    ..ParseLimits::default()
                })
            }))
            .chain([ParseLimits::default()]);
        for limits in limits {
            let parsed = parse_canonical_phase_2i_rule_for_test(
                source.clone(),
                rule,
                ParseConfig { limits },
            )
            .unwrap();
            assert!(
                parsed.stats.parser_steps <= limits.fuel,
                "{rule:?}: {text}: {limits:?}"
            );
            assert!(parsed.stats.events_emitted <= u64::from(limits.max_events));
            assert!(parsed.stats.diagnostics_emitted <= u64::from(limits.max_diagnostics));
            assert!(parsed.stats.recovery_bytes <= u64::from(limits.max_recovery_bytes));
            validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed)
                .unwrap_or_else(|error| panic!("{rule:?}: {text}: {limits:?}: {error:?}"));
            assert_eq!(
                reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
                &text[..parsed.consumed.end.0 as usize]
            );
        }
    }
}
