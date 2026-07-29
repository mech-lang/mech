use mech_syntax::document::{
  DocumentId, NodeFlags, ParseConfig, ParseLimits, Revision, TextSnapshot, parse_document,
  reconstruct_source, validate_lossless,
};

fn parse_with_limits(text: &str, limits: ParseLimits) -> mech_syntax::document::SyntaxSnapshot {
  parse_document(
    TextSnapshot::new(DocumentId(88), Revision(0), text).unwrap(),
    ParseConfig { limits },
  )
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
  assert_eq!(snapshot.diagnostics.len(), 1);
  assert_eq!(
    snapshot.diagnostics.iter().next().unwrap().code.as_str(),
    "syntax/recovery-limit"
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
  assert!(snapshot.root.flags.contains(NodeFlags::ERROR));
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
