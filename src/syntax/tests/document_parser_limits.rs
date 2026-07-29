use mech_syntax::document::{
  DocumentId, NodeFlags, ParseConfig, ParseLimits, RecoveryAction, Revision, SyntaxKind,
  SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags, parse_document, reconstruct_source,
  validate_lossless,
};

fn parse_with_limits(text: &str, limits: ParseLimits) -> mech_syntax::document::SyntaxSnapshot {
  parse_document(
    TextSnapshot::new(DocumentId(88), Revision(0), text).unwrap(),
    ParseConfig { limits },
  )
}

fn nodes_of_kind(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
  let mut nodes = Vec::new();
  if root.kind() == kind {
    nodes.push(root.clone());
  }
  for child in root.children() {
    nodes.extend(nodes_of_kind(&child, kind));
  }
  nodes
}

fn resource_range(
  snapshot: &mech_syntax::document::SyntaxSnapshot,
) -> TextRange {
  let ranges = snapshot
    .diagnostics
    .iter()
    .filter_map(|diagnostic| match diagnostic.recovery.as_ref() {
      Some(RecoveryAction::ResourceLimit { range }) => Some(*range),
      _ => None,
    })
    .collect::<Vec<_>>();
  assert_eq!(ranges.len(), 1, "expected exactly one resource diagnostic");
  ranges[0]
}

fn assert_only_remainder_is_error(
  text: &str,
  snapshot: &mech_syntax::document::SyntaxSnapshot,
  error_start: u32,
) {
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);

  let expected = TextRange::new(
    TextSize::from_u32(error_start),
    snapshot.source.byte_len(),
  );
  assert_eq!(resource_range(snapshot), expected);
  let errors = nodes_of_kind(&snapshot.syntax(), SyntaxKind::Error);
  assert_eq!(errors.len(), 1);
  assert_eq!(errors[0].range(), expected);
  assert!(
    snapshot
      .syntax()
      .tokens()
      .into_iter()
      .filter(|token| !token.flags().contains(TokenFlags::ERROR))
      .all(|token| token.range().end <= expected.start),
    "ordinary tokens must not overlap the unparsed remainder"
  );
}

#[test]
fn nesting_limit_returns_a_complete_snapshot() {
  let text = "x := (((((((1\nlater prose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_nesting: 2,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert!(snapshot
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.code.as_str() == "syntax/nesting-limit"));
}

#[test]
fn fuel_limit_consumes_remainder_and_never_panics() {
  let text = "x := 123456789\nlater prose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      fuel: 3,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert_eq!(
    snapshot
      .diagnostics
      .iter()
      .filter(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit")
      .count(),
    1
  );
}

#[test]
fn diagnostic_limit_suppresses_cascades_and_later_excess() {
  let text = "x :=\n1. Next\n--------\ny :=\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_diagnostics: 1,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(snapshot.diagnostics.len(), 1);
  assert!(snapshot.stats.diagnostics_truncated);
}

#[test]
fn zero_diagnostic_limit_records_suppressed_resource_diagnostic() {
  let snapshot = parse_with_limits(
    "ordinary prose\n",
    ParseLimits {
      max_diagnostics: 0,
      fuel: 0,
      ..ParseLimits::default()
    },
  );
  assert!(snapshot.diagnostics.is_empty());
  assert!(snapshot.stats.diagnostics_truncated);
}

#[test]
fn every_small_fuel_budget_terminates_inside_token_scanners() {
  for text in [
    "ordinary prose with many characters\n",
    "identifier := 123_456u64\n",
    "typed<record<field: u8>> := 1\n",
    "-- a long comment that exhausts fuel\n",
    "```\nopaque fence content that exhausts fuel\n```\n",
  ] {
    for fuel in 0..32 {
      let snapshot = parse_with_limits(
        text,
        ParseLimits {
          fuel,
          ..ParseLimits::default()
        },
      );
      validate_lossless(&snapshot.root, &snapshot.source).unwrap();
      assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
      assert!(snapshot.stats.parser_steps <= fuel);
    }
  }
}

#[test]
fn event_limit_produces_structural_resource_error() {
  let text = "ordinary prose with many tokens\nx := 1\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_events: 8,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert!(snapshot
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit"));
  assert!(snapshot
    .root
    .flags
    .intersects(NodeFlags::ERROR | NodeFlags::CONTAINS_ERROR));
  assert!(snapshot.stats.events_emitted <= 8);
}

#[test]
fn every_event_budget_is_a_hard_bound() {
  let text = "x := (((1 + @\n1. Later\n--------\nprose\n";
  for maximum in 0..32 {
    let snapshot = parse_with_limits(
      text,
      ParseLimits {
        max_events: maximum,
        ..ParseLimits::default()
      },
    );
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
    assert!(
      snapshot.stats.events_emitted <= u64::from(maximum),
      "{} events exceeded configured maximum {maximum}",
      snapshot.stats.events_emitted
    );
  }
}

#[test]
fn recovery_byte_limit_abandons_to_resource_error_without_losing_bytes() {
  let text = "x := @@@@@@@@@@\nlater prose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_recovery_bytes: 2,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert!(snapshot
    .diagnostics
    .iter()
    .any(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit"));
}

#[test]
fn completed_prefix_survives_middle_fuel_and_event_exhaustion() {
  let text = "1. Stable\n--------\n\nA stable paragraph.\n\nx := 1\n\nfinal-item-with-enough-content-to-exhaust-the-limit\n";
  let prefix_end = text.find("final-item").unwrap();
  for limits in [
    ParseLimits {
      fuel: 49,
      ..ParseLimits::default()
    },
    ParseLimits {
      max_events: 78,
      ..ParseLimits::default()
    },
  ] {
    let snapshot = parse_with_limits(text, limits);
    assert_only_remainder_is_error(text, &snapshot, prefix_end as u32);
    assert!(snapshot.stats.parser_steps <= limits.fuel);
    assert!(snapshot.stats.events_emitted <= u64::from(limits.max_events));
    let prefix_end = TextSize::from_u32(prefix_end as u32);
    for kind in [
      SyntaxKind::UlSubtitle,
      SyntaxKind::Paragraph,
      SyntaxKind::VariableDefine,
    ] {
      let completed = nodes_of_kind(&snapshot.syntax(), kind)
        .into_iter()
        .filter(|node| node.range().end <= prefix_end)
        .collect::<Vec<_>>();
      assert_eq!(
        completed.len(),
        1,
        "completed {kind:?} prefix node was not preserved"
      );
      assert!(
        completed[0]
          .tokens()
          .into_iter()
          .all(|token| !token.flags().contains(TokenFlags::ERROR)),
        "completed {kind:?} bytes must remain ordinary tokens"
      );
    }
  }
}

#[test]
fn early_middle_and_late_fuel_exhaustion_have_exact_remainders() {
  let text = "1. Stable\n--------\n\nA stable paragraph.\n\nx := 1\n\nfinal-item-with-enough-content-to-exhaust-the-limit\n";
  for (fuel, error_start) in [(0, 0), (49, 49), (100, 100)] {
    let snapshot = parse_with_limits(
      text,
      ParseLimits {
        fuel,
        ..ParseLimits::default()
      },
    );
    assert_only_remainder_is_error(text, &snapshot, error_start);
  }
}

#[test]
fn scanner_exhaustion_never_promotes_partially_scanned_bytes() {
  for (text, fuel, token_start) in [
    ("ordinaryparagraphtext\n", 10, 0),
    ("longidentifier := 1\n", 7, 0),
    ("x := 123_456u64\n", 9, 5),
    ("x<record<field: u8>> := 1\n", 10, 2),
    ("-- long comment contents\n", 10, 0),
    ("```\nlongfencecontent\n```\n", 8, 4),
  ] {
    let snapshot = parse_with_limits(
      text,
      ParseLimits {
        fuel,
        ..ParseLimits::default()
      },
    );
    assert_only_remainder_is_error(text, &snapshot, token_start);
    assert_eq!(snapshot.stats.parser_steps, fuel);
  }
}

#[test]
fn event_exhaustion_closes_a_deep_open_marker_stack_once() {
  let text = "x := ((((((((((((((((((((((((((((((((1\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_events: 24,
      ..ParseLimits::default()
    },
  );
  validate_lossless(&snapshot.root, &snapshot.source).unwrap();
  assert_eq!(reconstruct_source(&snapshot.root, &snapshot.source).unwrap(), text);
  assert_eq!(
    snapshot
      .diagnostics
      .iter()
      .filter(|diagnostic| diagnostic.code.as_str() == "syntax/recovery-limit")
      .count(),
    1
  );
  assert!(snapshot.stats.events_emitted <= 24);
}

#[test]
fn tiny_event_budget_uses_the_documented_whole_range_fallback() {
  let text = "1. Stable\n--------\nprose\n";
  let snapshot = parse_with_limits(
    text,
    ParseLimits {
      max_events: 6,
      ..ParseLimits::default()
    },
  );
  assert_only_remainder_is_error(text, &snapshot, 0);
  assert_eq!(snapshot.stats.events_emitted, 0);
  assert_eq!(nodes_of_kind(&snapshot.syntax(), SyntaxKind::UlSubtitle).len(), 0);
}
