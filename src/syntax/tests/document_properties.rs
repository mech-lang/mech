use mech_syntax::document::{
    DiagnosticAnchor, DocumentId, NodeFlags, ParseConfig, ParseLimits, Revision, SyntaxKind,
    SyntaxNode, TextSize, TextSnapshot, TokenFlags, parse_canonical_document, reconstruct_source,
    validate_lossless,
};
use proptest::prelude::*;

fn unicode_string(max_chars: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..=max_chars)
        .prop_map(|characters| characters.into_iter().collect())
}

fn parse_with_limits(text: &str, limits: ParseLimits) -> mech_syntax::document::SyntaxSnapshot {
    parse_canonical_document(
        TextSnapshot::new(DocumentId(100), Revision(0), text).unwrap(),
        ParseConfig { limits },
    )
}

fn count_kind(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|child| count_kind(&child, kind))
            .sum::<usize>()
}

// A canonical expression can continue across physical lines. Arbitrary mutations
// therefore cannot promise a later heading merely because a newline follows them.
// Comment owners, unlike expressions or unterminated string/delimiter owners, are
// physically line bounded. Prefix each original line without removing mutation
// bytes or normalizing CR, LF, or CRLF so this property has an independent boundary.
fn comment_owned_mutation(prefix: &str) -> String {
    let mut owned = String::from("-- ");
    let mut characters = prefix.chars().peekable();
    while let Some(character) = characters.next() {
        owned.push(character);
        if character == '\r' && characters.peek() == Some(&'\n') {
            owned.push(characters.next().unwrap());
        }
        if matches!(character, '\r' | '\n') {
            owned.push_str("-- ");
        }
    }
    owned
}

fn assert_node_invariants(node: &SyntaxNode, source_len: TextSize) {
    let range = node.range();
    assert!(range.start.0 <= range.end.0);
    assert!(range.end.0 <= source_len.0);
    if node.flags().contains(NodeFlags::MISSING) {
        assert!(range.is_empty());
    }
    if node.flags().contains(NodeFlags::ERROR) {
        let token_bytes = node
            .tokens()
            .iter()
            .filter(|token| !token.flags().contains(TokenFlags::SYNTHETIC))
            .map(|token| token.range().len().0)
            .sum::<u32>();
        assert_eq!(token_bytes, range.len().0);
    }
    for child in node.children() {
        assert_node_invariants(&child, source_len);
    }
}

fn assert_snapshot_invariants(
    text: &str,
    snapshot: &mech_syntax::document::SyntaxSnapshot,
    limits: ParseLimits,
) {
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
    assert_eq!(snapshot.root.text_len, TextSize(text.len() as u32));
    assert!(snapshot.stats.parser_steps <= limits.fuel);
    assert!(snapshot.stats.events_emitted <= u64::from(limits.max_events));
    assert!(snapshot.diagnostics.len() <= limits.max_diagnostics as usize);
    assert_node_invariants(&snapshot.syntax(), snapshot.source.byte_len());

    let mut covered = TextSize::ZERO;
    for token in snapshot.syntax().tokens() {
        if token.flags().contains(TokenFlags::SYNTHETIC) {
            assert!(token.range().is_empty());
        } else {
            assert_eq!(token.range().start, covered);
            covered = token.range().end;
        }
    }
    assert_eq!(covered, snapshot.source.byte_len());

    for diagnostic in snapshot.diagnostics.iter() {
        if let Some(rule) = diagnostic.rule {
            assert!(
                mech_syntax::document::parser::canonical_rule_name(rule).is_some(),
                "parser diagnostic used a RuleId outside the canonical rule registry"
            );
        }
        if let Some(mech_syntax::document::RecoveryAction::Abandon { rule, .. }) =
            diagnostic.recovery.as_ref()
        {
            assert!(
                mech_syntax::document::parser::canonical_rule_name(*rule).is_some(),
                "abandon recovery used a RuleId outside the canonical rule registry"
            );
        }
        let range = diagnostic
            .primary
            .resolve(snapshot.revision, &snapshot.nodes)
            .expect("diagnostic anchor must resolve in its revision");
        assert!(range.end.0 <= snapshot.source.byte_len().0);
        for label in &diagnostic.labels {
            if let Some(range) = label.anchor.resolve(snapshot.revision, &snapshot.nodes) {
                assert!(range.end.0 <= snapshot.source.byte_len().0);
            }
        }
        for fix in &diagnostic.fixes {
            for edit in &fix.edits {
                assert!(edit.delete.end.0 <= snapshot.source.byte_len().0);
            }
        }
        if let DiagnosticAnchor::Absolute { revision, .. } = diagnostic.primary {
            assert_eq!(revision, snapshot.revision);
        }
    }
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(256))]

  #[test]
  fn arbitrary_utf8_is_total_lossless_and_bounded(text in unicode_string(192)) {
    let limits = ParseLimits {
      max_nesting: 32,
      max_diagnostics: 24,
      max_events: 20_000,
      max_recovery_bytes: 2_048,
      fuel: 100_000,
    };
    let snapshot = parse_with_limits(&text, limits);
    assert_snapshot_invariants(&text, &snapshot, limits);
  }

  #[test]
  fn diagnostic_limit_is_never_exceeded(
    text in unicode_string(128),
    maximum in 0_u32..12,
  ) {
    let limits = ParseLimits {
      max_diagnostics: maximum,
      ..ParseLimits::default()
    };
    let snapshot = parse_with_limits(&text, limits);
    prop_assert!(snapshot.diagnostics.len() <= maximum as usize);
    prop_assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  }

  #[test]
  fn later_heading_survives_comment_owned_utf8_mutation(prefix in unicode_string(96)) {
    let owned = comment_owned_mutation(&prefix);
    let text = format!(
      "x :=;\n{owned}\n1. Stable Heading\n----------------\nlater paragraph\n"
    );
    let limits = ParseLimits::default();
    let snapshot = parse_with_limits(&text, limits);
    assert_snapshot_invariants(&text, &snapshot, limits);
    prop_assert!(snapshot.diagnostics.iter().any(|diagnostic|
      diagnostic.code.as_str() == "syntax/missing-variable-definition-value"
    ));
    prop_assert_eq!(count_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle), 1);
  }
}

#[test]
fn canonical_newline_continuation_seed_has_an_owned_restart_control() {
    let limits = ParseLimits::default();
    let raw = "x := ¡_\n1. Stable Heading\n----------------\nlater paragraph\n";
    let raw_snapshot = parse_with_limits(raw, limits);
    assert_snapshot_invariants(raw, &raw_snapshot, limits);
    assert_eq!(
        count_kind(&raw_snapshot.syntax(), SyntaxKind::UlSubtitle),
        0
    );
    let error = raw_snapshot
        .syntax()
        .children()
        .find(|node| node.kind() == SyntaxKind::Error)
        .expect("canonical continuation leaves the unaccepted document tail owned by Error");
    assert_eq!(
        error.text().unwrap(),
        ":= ¡_\n1. Stable Heading\n----------------\nlater paragraph\n"
    );

    for prefix in ["¡_", "\"( [ { ```", "one\rtwo\nthree\r\nfour"] {
        let protected = format!(
            "x :=;\n{}\n1. Stable Heading\n----------------\nlater paragraph\n",
            comment_owned_mutation(prefix)
        );
        let protected_snapshot = parse_with_limits(&protected, limits);
        assert_snapshot_invariants(&protected, &protected_snapshot, limits);
        assert_eq!(
            count_kind(&protected_snapshot.syntax(), SyntaxKind::UlSubtitle),
            1
        );
        assert!(protected_snapshot.diagnostics.iter().any(
            |diagnostic| diagnostic.code.as_str() == "syntax/missing-variable-definition-value"
        ));
    }
    assert_eq!(
        comment_owned_mutation("one\rtwo\nthree\r\nfour"),
        "-- one\r-- two\n-- three\r\n-- four"
    );
}

#[test]
fn maximum_configured_nesting_and_diagnostics_are_exercised() {
    let limits = ParseLimits {
        max_nesting: 8,
        max_diagnostics: 4,
        ..ParseLimits::default()
    };
    let text = "x := (((((((((((1\n1. Next\n--------\ny :=\nz :=\n";
    let snapshot = parse_with_limits(text, limits);
    assert_snapshot_invariants(text, &snapshot, limits);
    assert!(snapshot.diagnostics.len() <= 4);
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "syntax/nesting-limit")
    );
}

#[test]
fn cr_lf_crlf_and_unicode_ranges_remain_exact() {
    let text = "💡 := 1 +\rnext\r\nfinal\n";
    let limits = ParseLimits::default();
    let snapshot = parse_with_limits(text, limits);
    assert_snapshot_invariants(text, &snapshot, limits);
    assert_eq!(
        snapshot.source.line_index().line_starts(),
        &[TextSize(0), TextSize(12), TextSize(18), TextSize(24)]
    );
}
